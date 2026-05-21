use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
};
use chrono::Utc;
use url::Url;
use uuid::Uuid;

use crate::{
    provider::{
        fetch::fetch_gas_price, simulate::simulate_transaction, tx::send_relay_transaction,
    },
    utils::{
        callback::fire_callback,
        callback_security,
        errors::rpc_errors::{
            insufficient_balance_error, invalid_params_error, quote_expired_error,
            simulation_failed_error, transaction_too_large_error, unsupported_capability_error,
            unsupported_chain_error, unsupported_payment_token_error,
        },
        hex::parse_hex_u256,
        misc::{stub_mode_enabled, validate_authorization_list},
        task::resolve_task_id,
    },
    Config, RelayerRequest, RequestStatus, SendTransactionParams, SpecStatusResponse, Storage,
};

/// Core logic shared by sendTransaction and sendTransactionMultichain.
/// Returns the task_id on success.
pub async fn process_single_transaction(
    storage: &Storage,
    params: &SendTransactionParams,
    cfg: &Config,
) -> Result<String, jsonrpc_core::Error> {
    if params.to.is_empty() {
        return Err(invalid_params_error());
    }
    if params.data.is_empty() {
        return Err(invalid_params_error());
    }
    if params.chain_id.is_empty() {
        return Err(invalid_params_error());
    }

    let chain_id: u64 = params
        .chain_id
        .parse()
        .map_err(|_| invalid_params_error())?;

    if !cfg.is_chain_supported(chain_id) {
        tracing::warn!("Unsupported chain: {}", chain_id);
        return Err(unsupported_chain_error());
    }

    // 4207: calldata larger than 128 KB is rejected to prevent relay abuse.
    const MAX_CALLDATA_BYTES: usize = 128 * 1024;
    let raw_data = params.data.strip_prefix("0x").unwrap_or(&params.data);
    if raw_data.len() / 2 > MAX_CALLDATA_BYTES {
        tracing::warn!("Calldata too large: {} bytes", raw_data.len() / 2);
        return Err(transaction_too_large_error());
    }

    // 4204: if the caller embedded a quote expiry in context, enforce it.
    if let Some(expiry) = params
        .context
        .as_ref()
        .and_then(|ctx| ctx.get("expiry"))
        .and_then(|v| v.as_u64())
    {
        let now = chrono::Utc::now().timestamp() as u64;
        if now > expiry {
            tracing::warn!("Quote expired at {}, now {}", expiry, now);
            return Err(quote_expired_error());
        }
    }

    let wallet_address: Address = params.to.parse().map_err(|_| invalid_params_error())?;

    if let Some(auth_list) = &params.authorization_list {
        if !auth_list.is_empty() {
            validate_authorization_list(auth_list, chain_id, wallet_address)?;
        }
    }

    // Validate payment
    match params.payment.payment_type.as_str() {
        "token" => {
            let token_addr = params
                .payment
                .address
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if token_addr.is_empty() {
                return Err(invalid_params_error());
            }

            let zero_addr = "0x0000000000000000000000000000000000000000";
            if token_addr != zero_addr {
                // ERC20 - verify it's supported
                let supported_tokens = cfg.get_supported_tokens();
                if !supported_tokens
                    .iter()
                    .any(|t| t.to_ascii_lowercase() == token_addr)
                {
                    return Err(unsupported_payment_token_error());
                }
            }
        }
        "sponsored" => {}
        _ => return Err(unsupported_capability_error()),
    }

    let gas_price = fetch_gas_price(chain_id, cfg)
        .await
        .unwrap_or_else(|_| "0x4a817c800".to_string());

    let sim_gas = match simulate_transaction(&params.to, &params.data, chain_id, cfg).await {
        Ok(gas) => gas,
        Err(e) => {
            if cfg.is_simulation_disabled() {
                150_000
            } else {
                tracing::warn!("Simulation failed for {}: {}", params.to, e);
                return Err(simulation_failed_error());
            }
        }
    };

    // For native token payment, check the wallet has sufficient balance to cover gas
    if params.payment.payment_type == "token" {
        let token_addr = params
            .payment
            .address
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let zero_addr = "0x0000000000000000000000000000000000000000";

        if token_addr == zero_addr && !stub_mode_enabled() {
            let gas_price_u256 =
                parse_hex_u256(&gas_price).ok_or_else(jsonrpc_core::Error::internal_error)?;
            let required = gas_price_u256
                .checked_mul(U256::from(sim_gas))
                .ok_or_else(jsonrpc_core::Error::internal_error)?;

            if let Some(rpc_url) = cfg.rpc_url_for_chain(&chain_id.to_string()) {
                if let Ok(endpoint) = Url::parse(&rpc_url) {
                    let provider = ProviderBuilder::new().on_http(endpoint);
                    if let Ok(balance) = provider.get_balance(wallet_address).await {
                        if balance < required {
                            return Err(insufficient_balance_error());
                        }
                    }
                }
            }
        }
    }

    let fee_collector = std::env::var("RELAYX_FEE_COLLECTOR")
        .ok()
        .or_else(|| cfg.fee_collector())
        .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string());

    // Resolve task ID (generate or validate client-provided)
    let task_id = resolve_task_id(params.task_id.as_deref(), storage)?;

    // Extract optional callback URL from context.callbackUrl
    let callback_url = params
        .context
        .as_ref()
        .and_then(|ctx| ctx.get("callbackUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if let Some(ref u) = callback_url {
        callback_security::validate_outbound_webhook_url(u)
            .await
            .map_err(|_| invalid_params_error())?;
    }

    let internal_id = Uuid::new_v4();

    let relayer_request = RelayerRequest {
        id: internal_id,
        task_id: task_id.clone(),
        from_address: fee_collector.clone(),
        to_address: params.to.clone(),
        amount: "0".to_string(),
        gas_limit: sim_gas,
        gas_price: gas_price.clone(),
        data: Some(params.data.clone()),
        nonce: 0,
        chain_id,
        transaction_hash: None,
        status: RequestStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        error_message: None,
        callback_url,
    };

    storage
        .create_request(relayer_request.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to store request: {}", e);
            jsonrpc_core::Error::internal_error()
        })?;

    tracing::info!(
        "Transaction accepted - task_id: {}, chain: {}",
        task_id,
        chain_id
    );

    match send_relay_transaction(&params.to, &params.data, chain_id, sim_gas, &gas_price, cfg).await
    {
        Ok(tx_hash) => {
            if let Err(e) = storage
                .update_request_tx_hash(internal_id, tx_hash.clone())
                .await
            {
                tracing::warn!("Failed to store tx hash: {}", e);
            }
            if let Err(e) = storage
                .update_request_status(internal_id, RequestStatus::Processing, None)
                .await
            {
                tracing::warn!("Failed to set Processing status: {}", e);
            }
            if stub_mode_enabled() {
                let _ = storage
                    .update_request_status(internal_id, RequestStatus::Completed, None)
                    .await;
            }
            tracing::info!("Relay sent - hash: {}, task_id: {}", tx_hash, task_id);
        }
        Err(e) => {
            tracing::error!("Relay failed for task_id {}: {}", task_id, e);
            sentry::capture_message(&format!("Relay failed: {}", e), sentry::Level::Error);
            let _ = storage
                .update_request_status(internal_id, RequestStatus::Failed, Some(e.clone()))
                .await;
            if relayer_request.callback_url.is_some() {
                let status_resp = SpecStatusResponse {
                    chain_id: chain_id.to_string(),
                    created_at: relayer_request.created_at.timestamp() as u64,
                    status: 400,
                    hash: None,
                    receipt: None,
                    message: Some(e),
                    data: None,
                };
                fire_callback(&relayer_request, &status_resp, cfg).await;
            }
        }
    }

    Ok(task_id)
}
