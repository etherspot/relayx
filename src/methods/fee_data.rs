use alloy::{
    primitives::{Address, Bytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
use chrono::Utc;
use url::Url;

use crate::{
    provider::fetch::fetch_gas_fees,
    utils::{
        errors::rpc_errors::{
            invalid_params_error, unsupported_chain_error, unsupported_payment_token_error,
        },
        hex::hex_to_decimal,
        misc::stub_mode_enabled,
    },
    Config, FeeDataParams, FeeDataResponse, GasFees, TokenDetails,
};

/// Build a spec-compliant relayer_getFeeData response.
pub async fn build_fee_data_response(
    cfg: &Config,
    req: &FeeDataParams,
) -> Result<FeeDataResponse, jsonrpc_core::Error> {
    tracing::debug!(
        "Building fee data for token: {} on chain: {}",
        req.token,
        req.chain_id
    );

    let now = Utc::now().timestamp() as u64;
    let expiry = now + 600;

    let chain_id: u64 = req.chain_id.parse().map_err(|_| invalid_params_error())?;

    if !cfg.is_chain_supported(chain_id) {
        return Err(unsupported_chain_error());
    }

    let fees = fetch_gas_fees(chain_id, cfg)
        .await
        .unwrap_or_else(|_| GasFees {
            gas_price: "0x4a817c800".to_string(),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        });

    let fee_collector = cfg.fee_collector_for_chain(&chain_id.to_string());
    let zero_addr = "0x0000000000000000000000000000000000000000";

    if req.token.to_lowercase() == zero_addr {
        // Native token: rate is 1.0 (you pay in the native currency itself)
        return Ok(FeeDataResponse {
            chain_id: req.chain_id.clone(),
            token: TokenDetails {
                address: zero_addr.to_string(),
                decimals: 18,
            },
            rate: 1.0,
            min_fee: None,
            expiry,
            gas_price: hex_to_decimal(&fees.gas_price),
            max_fee_per_gas: fees.max_fee_per_gas.clone(),
            max_priority_fee_per_gas: fees.max_priority_fee_per_gas.clone(),
            fee_collector: fee_collector.clone(),
            context: None,
        });
    }

    if stub_mode_enabled() {
        let token_decimals = cfg
            .chainlink_token_decimals(&chain_id.to_string(), &req.token)
            .unwrap_or(18);
        return Ok(FeeDataResponse {
            chain_id: req.chain_id.clone(),
            token: TokenDetails {
                address: req.token.clone(),
                decimals: token_decimals,
            },
            // Stub: 1 ETH = 2000 tokens
            rate: 2000.0,
            min_fee: None,
            expiry,
            gas_price: hex_to_decimal(&fees.gas_price),
            max_fee_per_gas: fees.max_fee_per_gas.clone(),
            max_priority_fee_per_gas: fees.max_priority_fee_per_gas.clone(),
            fee_collector: fee_collector.clone(),
            context: None,
        });
    }

    // ERC20: compute rate = native_usd / token_usd (tokens per 1 native)
    let chain_str = chain_id.to_string();
    let token_feed = cfg.chainlink_token_usd(&chain_str, &req.token);
    let native_feed = cfg.chainlink_native_usd(&chain_str);

    if token_feed.is_none() || native_feed.is_none() {
        return Err(unsupported_payment_token_error());
    }

    let token_feed_addr = token_feed.unwrap();
    let native_feed_addr = native_feed.unwrap();

    let rpc_url = cfg
        .rpc_url_for_chain(&chain_str)
        .ok_or_else(unsupported_chain_error)?;

    async fn eth_call_bytes(rpc_url: &str, to_address: &str, calldata: &[u8]) -> Option<Vec<u8>> {
        let provider = ProviderBuilder::new().on_hyper_http(Url::parse(rpc_url).ok()?);
        let to: Address = to_address.parse().ok()?;
        let tx = TransactionRequest::default()
            .to(to)
            .input(Bytes::from(calldata.to_vec()).into());
        provider.call(&tx).await.ok().map(|bytes| bytes.to_vec())
    }

    async fn read_decimals(rpc_url: &str, contract: &str) -> Option<u8> {
        let sel: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
        eth_call_bytes(rpc_url, contract, &sel)
            .await?
            .last()
            .cloned()
    }

    async fn read_latest_answer(rpc_url: &str, aggregator: &str) -> Option<i128> {
        let sel: [u8; 4] = [0x50, 0xd2, 0x5b, 0xcd];
        let out = eth_call_bytes(rpc_url, aggregator, &sel).await?;
        if out.len() < 32 {
            return None;
        }
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&out[16..32]);
        Some(i128::from_be_bytes(buf))
    }

    let native_dec = read_decimals(&rpc_url, &native_feed_addr)
        .await
        .unwrap_or(8);
    let token_dec = read_decimals(&rpc_url, &token_feed_addr).await.unwrap_or(8);
    let native_px = read_latest_answer(&rpc_url, &native_feed_addr).await;
    let token_px = read_latest_answer(&rpc_url, &token_feed_addr).await;

    let (native_usd, token_usd) = match (native_px, token_px) {
        (Some(n), Some(t)) if n > 0 && t > 0 => (
            n as f64 / 10f64.powi(native_dec as i32),
            t as f64 / 10f64.powi(token_dec as i32),
        ),
        _ => return Err(unsupported_payment_token_error()),
    };

    // rate = tokens per 1 native = native_usd / token_usd
    let rate = native_usd / token_usd;

    async fn read_erc20_decimals(rpc_url: &str, token: &str) -> Option<u8> {
        let sel: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
        eth_call_bytes(rpc_url, token, &sel).await?.last().cloned()
    }
    let token_decimals =
        if let Some(decimals) = cfg.chainlink_token_decimals(&chain_str, &req.token) {
            decimals
        } else {
            read_erc20_decimals(&rpc_url, &req.token)
                .await
                .unwrap_or(18)
        };

    Ok(FeeDataResponse {
        chain_id: req.chain_id.clone(),
        token: TokenDetails {
            address: req.token.clone(),
            decimals: token_decimals,
        },
        rate,
        min_fee: None,
        expiry,
        gas_price: hex_to_decimal(&fees.gas_price),
        max_fee_per_gas: fees.max_fee_per_gas,
        max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
        fee_collector,
        context: None,
    })
}
