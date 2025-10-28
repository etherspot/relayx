use std::net::SocketAddr;

use alloy::{
    hex,
    json_abi::JsonAbi,
    network::EthereumWallet,
    primitives::{Address, Bytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use chrono::Utc;
use jsonrpc_core::{IoHandler, Params};
use jsonrpc_http_server::ServerBuilder;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::{
    config::Config,
    storage::Storage,
    types::{
        Capabilities, Erc20Payment, ExchangeRateError, ExchangeRateErrorBody, ExchangeRateQuote,
        ExchangeRateRequest, ExchangeRateResponse, ExchangeRateResultItem, ExchangeRateSuccess,
        GetCapabilitiesResponse, GetStatusRequest, GetStatusResponse, HealthResponse, Log,
        MultichainTransactionResult, NativePayment, OffchainFailure, OnchainFailure, Payment,
        PaymentType, QuoteInner, QuoteRequest, QuoteResponse, Receipt, RelayerCall, RelayerRequest,
        RequestStatus, Resubmission, SendTransactionMultichainRequest,
        SendTransactionMultichainResponse, SendTransactionRequest, SendTransactionResponse,
        SendTransactionResult, SponsoredPayment, StatusResult, TokenInfo,
    },
};

pub struct RpcServer {
    host: String,
    port: u16,
    storage: Storage,
    config: Config,
}

/// Load the wallet ABI from the JSON file
fn load_wallet_abi() -> Result<JsonAbi, anyhow::Error> {
    let abi_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("abi.json");

    let abi_content = std::fs::read_to_string(&abi_path)
        .map_err(|e| anyhow::anyhow!("Failed to read ABI file at {:?}: {}", abi_path, e))?;

    let abi_json: serde_json::Value = serde_json::from_str(&abi_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI JSON: {}", e))?;

    // Extract the 'abi' field from the JSON
    let abi_array = abi_json
        .get("abi")
        .ok_or_else(|| anyhow::anyhow!("ABI JSON missing 'abi' field"))?;

    let abi: JsonAbi = serde_json::from_value(abi_array.clone())
        .map_err(|e| anyhow::anyhow!("Failed to deserialize ABI: {}", e))?;

    Ok(abi)
}

/// Get the relayer's private key from environment variable
fn get_relayer_private_key() -> Result<String, String> {
    std::env::var("RELAYX_PRIVATE_KEY")
        .map_err(|_| "RELAYX_PRIVATE_KEY environment variable not set".to_string())
}

/// Fetch current gas price for the given chain.
/// Priority: Etherscan Gas Oracle (if API key configured) -> provider.get_gas_price
async fn fetch_gas_price(chain_id: u64, cfg: &Config) -> Result<String, String> {
    if let Some(api_key) = cfg.etherscan_api_key() {
        // Use Etherscan Gas Oracle
        // Docs: https://docs.etherscan.io/api-reference/endpoint/gasoracle
        let base = cfg.etherscan_api_base();
        let url = format!(
            "{}?chainid={}&module=gastracker&action=gasoracle&apikey={}",
            base, chain_id, api_key
        );
        match reqwest::Client::new().get(&url).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    if json.get("status").and_then(|s| s.as_str()) == Some("1") {
                        if let Some(result) = json.get("result") {
                            // Prefer ProposeGasPrice; values are in Gwei (string floats)
                            if let Some(p) = result.get("ProposeGasPrice").and_then(|v| v.as_str())
                            {
                                if let Ok(gwei_float) = p.parse::<f64>() {
                                    let wei = (gwei_float * 1e9_f64) as u128;
                                    tracing::debug!(
                                        "Etherscan gas price for chain {}: {} gwei ({} wei)",
                                        chain_id,
                                        gwei_float,
                                        wei
                                    );
                                    return Ok(format!("0x{:x}", wei));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Etherscan gasoracle JSON parse error: {}", e);
                }
            },
            Err(e) => {
                tracing::debug!("Etherscan gasoracle request failed: {}", e);
            }
        }
    }

    // Get RPC URL for the chain
    let rpc_url = cfg
        .rpc_url_for_chain(&chain_id.to_string())
        .ok_or_else(|| format!("No RPC URL configured for chain {}", chain_id))?;

    // Create provider for the chain
    let provider = ProviderBuilder::new().on_http(
        rpc_url
            .parse()
            .map_err(|e| format!("Invalid RPC URL: {}", e))?,
    );

    // Fallback: Fetch the current gas price from provider
    match provider.get_gas_price().await {
        Ok(gas_price) => {
            let gas_price_hex = format!("0x{:x}", gas_price);
            tracing::debug!(
                "Fetched gas price for chain {}: {} ({} wei)",
                chain_id,
                gas_price_hex,
                gas_price
            );
            Ok(gas_price_hex)
        }
        Err(e) => {
            let error_msg = format!("Failed to fetch gas price: {}", e);
            tracing::warn!("{}", error_msg);
            // Return default gas price on error
            Ok("0x4a817c800".to_string()) // 20 gwei fallback
        }
    }
}

/// Simple helper to bump hex gas price by given percent (e.g., 20 => +20%)
fn bump_gas_price_hex(gas_price_hex: &str, percent: u64) -> String {
    let s = gas_price_hex.strip_prefix("0x").unwrap_or(gas_price_hex);
    if let Ok(mut v) = u128::from_str_radix(s, 16) {
        v = v + (v * percent as u128 / 100u128);
        return format!("0x{:x}", v);
    }
    gas_price_hex.to_string()
}

/// Send a transaction on-chain by calling executeWithRelayer on the wallet
async fn send_relay_transaction(
    wallet_address: &str,
    calldata: &str,
    chain_id: u64,
    gas_limit: u64,
    gas_price_hex: &str,
    cfg: &Config,
) -> Result<String, String> {
    tracing::info!(
        "Preparing to send relay transaction to wallet {} on chain {}",
        wallet_address,
        chain_id
    );

    // Get private key for signing
    let private_key = get_relayer_private_key()?;

    // Parse private key and create signer
    let signer = private_key
        .parse::<PrivateKeySigner>()
        .map_err(|e| format!("Failed to parse private key: {}", e))?;

    let relayer_address = signer.address();
    tracing::debug!("Relayer address: 0x{:x}", relayer_address);

    // Create wallet from signer
    let wallet = EthereumWallet::from(signer);

    // Get RPC URL for the chain
    let rpc_url = cfg
        .rpc_url_for_chain(&chain_id.to_string())
        .ok_or_else(|| format!("No RPC URL configured for chain {}", chain_id))?;

    // Create provider with wallet for signing
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(
            rpc_url
                .parse()
                .map_err(|e| format!("Invalid RPC URL: {}", e))?,
        );

    // Parse wallet address
    let to_address: Address = wallet_address
        .parse()
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    // Parse calldata (should already be hex encoded)
    let calldata_bytes = if let Some(stripped) = calldata.strip_prefix("0x") {
        hex::decode(stripped).map_err(|e| format!("Invalid calldata hex: {}", e))?
    } else {
        hex::decode(calldata).map_err(|e| format!("Invalid calldata hex: {}", e))?
    };

    // Parse gas price
    let gas_price_value = if let Some(stripped) = gas_price_hex.strip_prefix("0x") {
        u128::from_str_radix(stripped, 16).map_err(|e| format!("Invalid gas price hex: {}", e))?
    } else {
        u128::from_str_radix(gas_price_hex, 16)
            .map_err(|e| format!("Invalid gas price hex: {}", e))?
    };

    // Get nonce for the relayer address
    let nonce = provider
        .get_transaction_count(relayer_address)
        .await
        .map_err(|e| format!("Failed to get nonce: {}", e))?;

    tracing::debug!(
        "Building transaction - Nonce: {}, Gas limit: {}, Gas price: {} wei",
        nonce,
        gas_limit,
        gas_price_value
    );

    // Build transaction
    let mut tx = TransactionRequest::default()
        .to(to_address)
        .input(calldata_bytes.into())
        .gas_limit(gas_limit);

    tx.nonce = Some(nonce);
    tx.gas_price = Some(gas_price_value);
    tx.chain_id = Some(chain_id);

    tracing::info!("Sending transaction to chain {}...", chain_id);

    // Send transaction
    match provider.send_transaction(tx).await {
        Ok(pending_tx) => {
            let tx_hash = *pending_tx.tx_hash();
            let tx_hash_hex = format!("0x{:x}", tx_hash);

            tracing::info!(
                "✓ Transaction sent successfully - Hash: {}, Chain: {}",
                tx_hash_hex,
                chain_id
            );

            Ok(tx_hash_hex)
        }
        Err(e) => {
            let error_msg = format!("Failed to send transaction: {}", e);
            tracing::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Simulate a transaction and estimate gas consumption
/// Returns the estimated gas on success
async fn simulate_transaction(
    wallet_address: &str,
    calldata: &str,
    chain_id: u64,
    cfg: &Config,
) -> Result<u64, String> {
    // Get RPC URL for the chain
    let rpc_url = cfg
        .rpc_url_for_chain(&chain_id.to_string())
        .ok_or_else(|| format!("No RPC URL configured for chain {}", chain_id))?;

    // Parse the wallet address
    let wallet_addr: Address = wallet_address
        .parse()
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    // Parse the calldata
    let calldata_bytes: Bytes = calldata
        .parse()
        .map_err(|e| format!("Invalid calldata format: {}", e))?;

    // Load the ABI and verify the function being called
    let abi = load_wallet_abi().map_err(|e| format!("Failed to load wallet ABI: {}", e))?;

    // Check if the calldata is calling executeWithRelayer
    // The first 4 bytes are the function selector
    if calldata_bytes.len() < 4 {
        return Err("Calldata too short".to_string());
    }

    let function_selector = &calldata_bytes[..4];

    // Find the executeWithRelayer function
    let execute_with_relayer_fn = abi
        .functions()
        .find(|f| f.name == "executeWithRelayer")
        .ok_or_else(|| "executeWithRelayer function not found in ABI".to_string())?;

    // Get the expected selector
    let expected_selector = execute_with_relayer_fn.selector();

    // Verify the selector matches
    if function_selector != expected_selector.as_slice() {
        return Err(format!(
            "Transaction is not calling executeWithRelayer (expected selector: 0x{}, got: 0x{})",
            hex::encode(expected_selector),
            hex::encode(function_selector)
        ));
    }

    // Create provider for the chain
    let provider = ProviderBuilder::new().on_http(
        rpc_url
            .parse()
            .map_err(|e| format!("Invalid RPC URL: {}", e))?,
    );

    // Create a transaction request for simulation
    let tx = TransactionRequest::default()
        .to(wallet_addr)
        .input(calldata_bytes.into());

    // First, simulate the transaction using eth_call to ensure it won't revert
    if let Err(e) = provider.call(&tx).await {
        let error_msg = format!("Transaction simulation failed: {}", e);
        tracing::warn!("{}", error_msg);
        return Err(error_msg);
    }

    // Now estimate the gas required for the transaction
    match provider.estimate_gas(&tx).await {
        Ok(gas_estimate) => {
            tracing::info!(
                "Transaction simulation succeeded for wallet {} on chain {}, estimated gas: {}",
                wallet_address,
                chain_id,
                gas_estimate
            );
            Ok(gas_estimate)
        }
        Err(e) => {
            let error_msg = format!("Gas estimation failed: {}", e);
            tracing::warn!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Endpoint business logic functions
async fn process_send_transaction(
    storage: Storage,
    input: &SendTransactionRequest,
    cfg: &Config,
) -> Result<SendTransactionResponse, jsonrpc_core::Error> {
    tracing::info!("=== relayer_sendTransaction request received ===");
    tracing::debug!(
        "Request details - To: {}, ChainId: {}, Payment: {}",
        input.to,
        input.chain_id,
        input.capabilities.payment.payment_type
    );

    // Validate the transaction request
    if input.to.is_empty() {
        tracing::warn!("Validation failed: Missing 'to' field");
        return Err(jsonrpc_core::Error::invalid_params(
            "Missing required field: 'to'",
        ));
    }

    if input.data.is_empty() {
        tracing::warn!("Validation failed: Missing 'data' field");
        return Err(jsonrpc_core::Error::invalid_params(
            "Missing required field: 'data'",
        ));
    }

    if input.chain_id.is_empty() {
        tracing::warn!("Validation failed: Missing 'chainId' field");
        return Err(jsonrpc_core::Error::invalid_params(
            "Missing required field: 'chainId'",
        ));
    }

    // Validate chain ID is a valid number
    let chain_id: u64 = input.chain_id.parse().map_err(|_| {
        tracing::warn!("Invalid chainId format: {}", input.chain_id);
        jsonrpc_core::Error::invalid_params("Invalid chainId: must be a valid number")
    })?;

    tracing::debug!("Validating chain support for chainId: {}", chain_id);

    // Check if chain is supported by the relayer
    if !cfg.is_chain_supported(chain_id) {
        tracing::warn!("Unsupported chain ID requested: {}", chain_id);
        return Err(jsonrpc_core::Error::invalid_params(format!(
            "Unsupported chain ID: {}",
            chain_id
        )));
    }

    tracing::debug!("Chain {} is supported", chain_id);

    // Fetch current gas price from the chain
    let gas_price = match fetch_gas_price(chain_id, cfg).await {
        Ok(price) => price,
        Err(e) => {
            tracing::warn!("Failed to fetch gas price, using default: {}", e);
            "0x4a817c800".to_string() // 20 gwei fallback
        }
    };

    tracing::debug!(
        "Validating payment capability: {}",
        input.capabilities.payment.payment_type
    );

    // Validate payment capability and estimate gas
    let gas_limit = match input.capabilities.payment.payment_type.as_str() {
        "native" => {
            tracing::debug!("Processing native payment transaction");

            // Validate native payment token address (should be zero address)
            if input.capabilities.payment.token != "0x0000000000000000000000000000000000000000" {
                tracing::warn!(
                    "Invalid native payment token address: {}",
                    input.capabilities.payment.token
                );
                return Err(jsonrpc_core::Error::invalid_params(
                    "Invalid native payment token address",
                ));
            }

            // Simulate transaction to ensure it will succeed and get gas estimate
            let gas = match simulate_transaction(&input.to, &input.data, chain_id, cfg).await {
                Ok(gas) => gas,
                Err(e) => {
                    tracing::error!(
                        "Transaction simulation failed for wallet {} on chain {}: {}",
                        input.to,
                        chain_id,
                        e
                    );
                    return Err(jsonrpc_core::Error::invalid_params(format!(
                        "Transaction simulation failed: {}",
                        e
                    )));
                }
            };

            tracing::info!(
                "Transaction simulation successful - Wallet: {}, Chain: {}, Estimated Gas: {}",
                input.to,
                chain_id,
                gas
            );
            gas
        }
        "erc20" => {
            tracing::debug!(
                "Processing ERC20 payment transaction with token: {}",
                input.capabilities.payment.token
            );

            // Validate ERC20 token address format
            if !input.capabilities.payment.token.starts_with("0x")
                || input.capabilities.payment.token.len() != 42
            {
                tracing::warn!(
                    "Invalid ERC20 token address format: {}",
                    input.capabilities.payment.token
                );
                return Err(jsonrpc_core::Error::invalid_params(
                    "Invalid ERC20 token address format",
                ));
            }

            tracing::debug!("ERC20 token address validated successfully");

            // Estimate gas for ERC20 transactions as well
            match simulate_transaction(&input.to, &input.data, chain_id, cfg).await {
                Ok(gas) => {
                    tracing::info!("ERC20 transaction gas estimate: {}", gas);
                    gas
                }
                Err(e) => {
                    tracing::warn!(
                        "ERC20 transaction simulation failed, using default gas limit: {}",
                        e
                    );
                    21000 // Use default if simulation fails
                }
            }
        }
        "sponsored" => {
            tracing::debug!("Processing sponsored transaction");

            // Estimate gas for sponsored transactions as well
            match simulate_transaction(&input.to, &input.data, chain_id, cfg).await {
                Ok(gas) => {
                    tracing::info!("Sponsored transaction gas estimate: {}", gas);
                    gas
                }
                Err(e) => {
                    tracing::warn!(
                        "Sponsored transaction simulation failed, using default gas limit: {}",
                        e
                    );
                    21000 // Use default if simulation fails
                }
            }
        }
        _ => {
            tracing::warn!(
                "Unsupported payment type: {}",
                input.capabilities.payment.payment_type
            );
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Unsupported payment type: {}",
                input.capabilities.payment.payment_type
            )));
        }
    };

    // Get fee collector address from config
    let fee_collector = std::env::var("RELAYX_FEE_COLLECTOR")
        .ok()
        .or_else(|| cfg.fee_collector())
        .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string());

    // Generate a unique transaction ID
    let transaction_id = Uuid::new_v4().to_string();

    tracing::info!("Generated transaction ID: {}", transaction_id);
    tracing::debug!(
        "Creating relayer request - Gas limit: {}, Gas price: {}, Fee collector: {}",
        gas_limit,
        gas_price,
        fee_collector
    );

    // Create a relayer request record
    let relayer_request = RelayerRequest {
        id: Uuid::parse_str(&transaction_id).unwrap(),
        from_address: fee_collector.clone(), // Use fee collector as sender address
        to_address: input.to.clone(),
        amount: "0".to_string(), // Will be calculated based on transaction
        gas_limit,               // Gas limit from simulation
        gas_price: gas_price.clone(), // Dynamic gas price from RPC
        data: Some(input.data.clone()),
        nonce: 0, // Will be fetched from chain
        chain_id,
        transaction_hash: None, // Will be set when transaction is sent
        status: RequestStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        error_message: None,
    };

    // Store the request in storage
    tracing::debug!("Storing transaction request in database");
    if let Err(e) = storage.create_request(relayer_request.clone()).await {
        tracing::error!("Failed to store transaction request: {}", e);
        return Err(jsonrpc_core::Error::internal_error());
    }

    tracing::debug!("Transaction request stored successfully");

    // Log the transaction request
    tracing::info!(
        "✓ Transaction accepted - ID: {}, To: {}, Chain: {}, Payment: {}, Gas: {}",
        transaction_id,
        input.to,
        chain_id,
        input.capabilities.payment.payment_type,
        gas_limit
    );

    // Send the transaction on-chain
    tracing::info!("Sending relay transaction on-chain...");
    match send_relay_transaction(
        &input.to,
        &input.data,
        chain_id,
        gas_limit,
        &gas_price,
        cfg,
    )
    .await
    {
        Ok(tx_hash) => {
            tracing::info!(
                "✓ Relay transaction sent successfully - TX Hash: {}, ID: {}",
                tx_hash,
                transaction_id
            );

            // Update storage with transaction hash and set status to Processing
            let mut updated_request = relayer_request;
            updated_request.status = RequestStatus::Processing;

            // store tx hash
            if let Err(e) = storage
                .update_request_tx_hash(updated_request.id, tx_hash.clone())
                .await
            {
                tracing::warn!("Failed to store tx hash: {}", e);
            }

            if let Err(e) = storage
                .update_request_status(updated_request.id, RequestStatus::Processing, None)
                .await
            {
                tracing::warn!("Failed to update request status to Processing: {}", e);
            }

            tracing::info!(
                "✓ Transaction relay complete - TX Hash: {}, ID: {}, Chain: {}",
                tx_hash,
                transaction_id,
                chain_id
            );
        }
        Err(e) => {
            tracing::error!(
                "Failed to send relay transaction for ID {}: {}",
                transaction_id,
                e
            );

            // Update storage with error status
            if let Err(update_err) = storage
                .update_request_status(relayer_request.id, RequestStatus::Failed, Some(e.clone()))
                .await
            {
                tracing::error!("Failed to update request status to Failed: {}", update_err);
            }

            return Err(jsonrpc_core::Error::internal_error());
        }
    }

    // Return the response with the generated transaction ID
    Ok(SendTransactionResponse {
        result: vec![SendTransactionResult {
            chain_id: input.chain_id.clone(),
            id: transaction_id,
        }],
    })
}

/// Process multichain transaction request
async fn process_send_transaction_multichain(
    storage: Storage,
    input: &SendTransactionMultichainRequest,
    cfg: &Config,
) -> Result<SendTransactionMultichainResponse, jsonrpc_core::Error> {
    tracing::info!("=== relayer_sendTransactionMultichain request received ===");
    tracing::debug!(
        "Request details - Transactions: {}, PaymentChainId: {}, Payment: {}",
        input.transactions.len(),
        input.payment_chain_id,
        input.capabilities.payment.payment_type
    );

    // Validate that we have at least one transaction
    if input.transactions.is_empty() {
        tracing::warn!("Validation failed: No transactions provided");
        return Err(jsonrpc_core::Error::invalid_params(
            "At least one transaction is required",
        ));
    }

    // Validate payment chain ID
    if input.payment_chain_id.is_empty() {
        tracing::warn!("Validation failed: Missing 'paymentChainId' field");
        return Err(jsonrpc_core::Error::invalid_params(
            "Missing required field: 'paymentChainId'",
        ));
    }

    let payment_chain_id: u64 = input.payment_chain_id.parse().map_err(|_| {
        tracing::warn!("Invalid paymentChainId format: {}", input.payment_chain_id);
        jsonrpc_core::Error::invalid_params("Invalid paymentChainId: must be a valid number")
    })?;

    // Validate payment chain is supported
    if !cfg.is_chain_supported(payment_chain_id) {
        tracing::warn!("Unsupported payment chain ID: {}", payment_chain_id);
        return Err(jsonrpc_core::Error::invalid_params(format!(
            "Unsupported payment chain ID: {}",
            payment_chain_id
        )));
    }

    tracing::debug!(
        "Validating payment capability: {}",
        input.capabilities.payment.payment_type
    );

    // Validate payment capability
    match input.capabilities.payment.payment_type.as_str() {
        "native" => {
            if input.capabilities.payment.token != "0x0000000000000000000000000000000000000000" {
                tracing::warn!(
                    "Invalid native payment token address: {}",
                    input.capabilities.payment.token
                );
                return Err(jsonrpc_core::Error::invalid_params(
                    "Invalid native payment token address",
                ));
            }
        }
        "erc20" => {
            if !input.capabilities.payment.token.starts_with("0x")
                || input.capabilities.payment.token.len() != 42
            {
                tracing::warn!(
                    "Invalid ERC20 token address format: {}",
                    input.capabilities.payment.token
                );
                return Err(jsonrpc_core::Error::invalid_params(
                    "Invalid ERC20 token address format",
                ));
            }
        }
        "sponsored" => {
            tracing::debug!("Processing sponsored multichain transaction");
        }
        _ => {
            tracing::warn!(
                "Unsupported payment type: {}",
                input.capabilities.payment.payment_type
            );
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Unsupported payment type: {}",
                input.capabilities.payment.payment_type
            )));
        }
    }

    // Get fee collector address from config (shared across all transactions)
    let fee_collector = std::env::var("RELAYX_FEE_COLLECTOR")
        .ok()
        .or_else(|| cfg.fee_collector())
        .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string());

    let mut results = Vec::new();

    // Process each transaction
    for (idx, tx) in input.transactions.iter().enumerate() {
        tracing::debug!(
            "Processing transaction {} of {}: ChainId: {}, To: {}",
            idx + 1,
            input.transactions.len(),
            tx.chain_id,
            tx.to
        );

        // Validate transaction fields
        if tx.to.is_empty() {
            tracing::warn!("Transaction {} missing 'to' field", idx);
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Transaction {}: Missing required field: 'to'",
                idx
            )));
        }

        if tx.data.is_empty() {
            tracing::warn!("Transaction {} missing 'data' field", idx);
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Transaction {}: Missing required field: 'data'",
                idx
            )));
        }

        if tx.chain_id.is_empty() {
            tracing::warn!("Transaction {} missing 'chainId' field", idx);
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Transaction {}: Missing required field: 'chainId'",
                idx
            )));
        }

        // Validate chain ID format and support
        let chain_id: u64 = tx.chain_id.parse().map_err(|_| {
            tracing::warn!("Transaction {} invalid chainId: {}", idx, tx.chain_id);
            jsonrpc_core::Error::invalid_params(format!(
                "Transaction {}: Invalid chainId format",
                idx
            ))
        })?;

        if !cfg.is_chain_supported(chain_id) {
            tracing::warn!("Transaction {} unsupported chain: {}", idx, chain_id);
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Transaction {}: Unsupported chain ID: {}",
                idx, chain_id
            )));
        }

        // Fetch current gas price from the chain for this transaction
        let gas_price = match fetch_gas_price(chain_id, cfg).await {
            Ok(price) => price,
            Err(e) => {
                tracing::warn!(
                    "Transaction {}: Failed to fetch gas price, using default: {}",
                    idx,
                    e
                );
                "0x4a817c800".to_string() // 20 gwei fallback
            }
        };

        // Estimate gas limit for this transaction
        let gas_limit = match simulate_transaction(&tx.to, &tx.data, chain_id, cfg).await {
            Ok(gas) => {
                tracing::debug!("Transaction {}: Estimated gas: {}", idx, gas);
                gas
            }
            Err(e) => {
                tracing::warn!(
                    "Transaction {}: Simulation failed, using default gas limit: {}",
                    idx,
                    e
                );
                21000 // Use default if simulation fails
            }
        };

        // Generate unique transaction ID
        let transaction_id = Uuid::new_v4().to_string();

        tracing::info!(
            "Transaction {}: Generated ID {} for chain {} (gas: {}, gasPrice: {})",
            idx,
            transaction_id,
            chain_id,
            gas_limit,
            gas_price
        );

        // Create relayer request record
        let relayer_request = RelayerRequest {
            id: Uuid::parse_str(&transaction_id).unwrap(),
            from_address: fee_collector.clone(), // Use fee collector as sender address
            to_address: tx.to.clone(),
            amount: "0".to_string(),
            gas_limit, // Dynamic gas limit from simulation
            gas_price: gas_price.clone(), // Dynamic gas price from RPC
            data: Some(tx.data.clone()),
            nonce: 0,
            chain_id,
            transaction_hash: None, // Will be set when transaction is sent
            status: RequestStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
        };

        // Store the request
        if let Err(e) = storage.create_request(relayer_request.clone()).await {
            tracing::error!("Failed to store transaction {} request: {}", idx, e);
            return Err(jsonrpc_core::Error::internal_error());
        }

        tracing::debug!("Transaction {} stored successfully", idx);

        // Send the transaction on-chain
        match send_relay_transaction(&tx.to, &tx.data, chain_id, gas_limit, &gas_price, cfg).await {
            Ok(tx_hash) => {
                tracing::info!(
                    "✓ Multichain relay sent - idx: {}, TX Hash: {}, ID: {}",
                    idx,
                    tx_hash,
                    transaction_id
                );
                // store tx hash and set Processing
                if let Err(e) = storage
                    .update_request_tx_hash(Uuid::parse_str(&transaction_id).unwrap(), tx_hash)
                    .await
                {
                    tracing::warn!("Transaction {}: failed to store tx hash: {}", idx, e);
                }
                if let Err(e) = storage
                    .update_request_status(
                        Uuid::parse_str(&transaction_id).unwrap(),
                        RequestStatus::Processing,
                        None,
                    )
                    .await
                {
                    tracing::warn!("Transaction {}: failed to set Processing: {}", idx, e);
                }
            }
            Err(e) => {
                tracing::error!("Transaction {}: failed to send: {}", idx, e);
                if let Err(update_err) = storage
                    .update_request_status(
                        Uuid::parse_str(&transaction_id).unwrap(),
                        RequestStatus::Failed,
                        Some(e.clone()),
                    )
                    .await
                {
                    tracing::warn!("Transaction {}: failed to set Failed: {}", idx, update_err);
                }
            }
        }

        // Add to results
        results.push(MultichainTransactionResult {
            chain_id: tx.chain_id.clone(),
            id: transaction_id,
        });
    }

    tracing::info!(
        "✓ Multichain transaction accepted - {} transaction(s) across {} chain(s), Payment chain: {}",
        results.len(),
        input.transactions.iter().map(|t| &t.chain_id).collect::<std::collections::HashSet<_>>().len(),
        input.payment_chain_id
    );

    Ok(SendTransactionMultichainResponse { result: results })
}

async fn process_get_status(
    storage: Storage,
    request: &GetStatusRequest,
    _cfg: &Config,
) -> Result<GetStatusResponse, jsonrpc_core::Error> {
    tracing::info!("=== relayer_getStatus request received ===");
    tracing::debug!("Querying status for {} transaction(s)", request.ids.len());

    let mut results: Vec<StatusResult> = Vec::new();

    for id in &request.ids {
        let mut status_result = StatusResult {
            version: "2.0.0".to_string(),
            id: id.clone(),
            status: 404,
            receipts: Vec::new(),
            resubmissions: Vec::new(),
            offchain_failure: Vec::new(),
            onchain_failure: Vec::new(),
        };

        match Uuid::parse_str(id) {
            Ok(uuid) => match storage.get_request(uuid).await {
                Ok(Some(req)) => {
                    // Map internal status to HTTP-style code
                    status_result.status = match req.status {
                        RequestStatus::Pending | RequestStatus::Processing => 201,
                        RequestStatus::Completed => 200,
                        RequestStatus::Failed => 500,
                    };

                    // If there was an off-chain error, include it
                    if let Some(msg) = req.error_message.clone() {
                        status_result.offchain_failure.push(OffchainFailure { message: msg });
                    }

                    // Include any resubmissions recorded
                    if let Ok(mut resubs) = storage.get_resubmissions(uuid).await {
                        // sort stable (optional)
                        status_result.resubmissions.append(&mut resubs);
                    }
                }
                Ok(None) => {
                    // keep 404
                }
                Err(e) => {
                    tracing::warn!("Failed to read request {}: {}", id, e);
                    status_result.status = 500;
                    status_result.offchain_failure.push(OffchainFailure {
                        message: "internal storage error".to_string(),
                    });
                }
            },
            Err(_) => {
                status_result.status = 400;
                status_result.offchain_failure.push(OffchainFailure {
                    message: "invalid id format".to_string(),
                });
            }
        }

        results.push(status_result);
    }

    tracing::info!(
        "✓ Status query completed for {} transaction(s)",
        results.len()
    );
    Ok(GetStatusResponse { result: results })
}

async fn process_health_check(
    storage: Storage,
    _cfg: &Config,
) -> Result<HealthResponse, jsonrpc_core::Error> {
    tracing::debug!("=== health_check request received ===");

    let total_requests = storage.get_total_request_count().await.map_err(|e| {
        tracing::error!("Failed to get total request count: {}", e);
        jsonrpc_core::Error::internal_error()
    })?;

    let pending_requests = storage
        .get_request_count_by_status(RequestStatus::Pending)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get pending request count: {}", e);
            jsonrpc_core::Error::internal_error()
        })?;

    let completed_requests = storage
        .get_request_count_by_status(RequestStatus::Completed)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get completed request count: {}", e);
            jsonrpc_core::Error::internal_error()
        })?;

    let failed_requests = storage
        .get_request_count_by_status(RequestStatus::Failed)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get failed request count: {}", e);
            jsonrpc_core::Error::internal_error()
        })?;

    tracing::debug!(
        "Health metrics - Total: {}, Pending: {}, Completed: {}, Failed: {}, Uptime: {}s",
        total_requests,
        pending_requests,
        completed_requests,
        failed_requests,
        storage.get_uptime_seconds()
    );

    Ok(build_health_response(
        storage.get_uptime_seconds(),
        total_requests,
        pending_requests,
        completed_requests,
        failed_requests,
    ))
}

async fn process_get_capabilities(
    _storage: Storage,
    cfg: &Config,
) -> Result<GetCapabilitiesResponse, jsonrpc_core::Error> {
    tracing::info!("=== relayer_getCapabilities request received ===");

    // Build capabilities based on configuration
    // Extract all supported tokens from the chainlink configuration
    let supported_tokens = cfg.get_supported_tokens();

    tracing::debug!(
        "Found {} supported token(s) from configuration",
        supported_tokens.len()
    );

    let mut payments = Vec::new();

    // Add ERC20 payment options for each supported token
    for token in &supported_tokens {
        tracing::debug!("Adding ERC20 payment capability for token: {}", token);
        payments.push(Payment::Erc20(Erc20Payment {
            payment_type: PaymentType::Erc20,
            token: token.clone(),
        }));
    }

    // If no tokens found in config, fall back to default token
    if payments.is_empty() {
        let default_token = cfg
            .default_token()
            .unwrap_or_else(|| "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string()); // USDC on Ethereum

        tracing::debug!(
            "No tokens configured, using default token: {}",
            default_token
        );
        payments.push(Payment::Erc20(Erc20Payment {
            payment_type: PaymentType::Erc20,
            token: default_token,
        }));
    }

    // Always include native payment option
    tracing::debug!("Adding native payment capability");
    payments.push(Payment::Native(NativePayment {
        payment_type: PaymentType::Native,
        token: "0x0000000000000000000000000000000000000000".to_string(),
    }));

    // Always include sponsored payment option
    tracing::debug!("Adding sponsored payment capability");
    payments.push(Payment::Sponsored(SponsoredPayment {
        payment_type: PaymentType::Sponsored,
    }));

    let capabilities = Capabilities { payment: payments };

    tracing::info!(
        "✓ Returning {} payment capability option(s)",
        capabilities.payment.len()
    );

    Ok(GetCapabilitiesResponse { capabilities })
}

// (unused) Kept for potential reuse; prefer cached path used in start()
// async fn process_get_exchange_rate(cfg: &Config, input: &ExchangeRateRequest) ->
// Result<ExchangeRateResponse, jsonrpc_core::Error> { 	let now = Utc::now().timestamp() as u64;
// 	let expiry = now + 600;
// 	Ok(build_exchange_rate_response_with_provider(cfg, &get_or_create_provider(cfg,
// &input.chain_id).await, input, expiry).await) }

// async fn process_get_quote(_cfg: &Config) -> Result<QuoteResponse, jsonrpc_core::Error> {
//     Ok(build_quote_response())
// }

/// Build a response for the health_check endpoint
fn build_health_response(
    uptime_seconds: u64,
    total_requests: u64,
    pending_requests: u64,
    completed_requests: u64,
    failed_requests: u64,
) -> HealthResponse {
    HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        uptime_seconds,
        total_requests,
        pending_requests,
        completed_requests,
        failed_requests,
    }
}

/// Build a dynamic response for the relayer_getExchangeRate endpoint
async fn build_exchange_rate_response(
    cfg: &Config,
    req: &ExchangeRateRequest,
) -> ExchangeRateResponse {
    tracing::debug!(
        "Building exchange rate response for token: {} on chain: {}",
        req.token,
        req.chain_id
    );

    let now = Utc::now().timestamp() as u64;
    let expiry = now + 600;

    let chain_id: u64 = match req.chain_id.parse() {
        Ok(v) => v,
        Err(_) => {
            return ExchangeRateResponse {
                result: vec![ExchangeRateResultItem::Error(ExchangeRateError {
                    error: ExchangeRateErrorBody {
                        id: req.chain_id.clone(),
                        message: "invalid chainId".to_string(),
                    },
                })],
            };
        }
    };

    // Zero address denotes native token
    let zero_addr = "0x0000000000000000000000000000000000000000".to_string();

    if req.token.to_lowercase() == zero_addr {
        // Native token: rate per gas = gasPrice (wei) / 1e18 ETH per gas
        let gas_price = fetch_gas_price(chain_id, cfg)
            .await
            .unwrap_or_else(|_| "0x4a817c800".to_string());
        let wei =
            u128::from_str_radix(gas_price.trim_start_matches("0x"), 16).unwrap_or(20_000_000_000);
        let rate_eth_per_gas = (wei as f64) / 1e18_f64;
        let item = ExchangeRateResultItem::Success(ExchangeRateSuccess {
            quote: ExchangeRateQuote {
                rate: rate_eth_per_gas,
                token: TokenInfo {
                    decimals: 18,
                    address: zero_addr.clone(),
                    symbol: Some("ETH".to_string()),
                    name: Some("Ethereum".to_string()),
                },
            },
            gas_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            fee_collector: std::env::var("RELAYX_FEE_COLLECTOR")
                .ok()
                .or_else(|| cfg.fee_collector())
                .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string()),
            expiry,
        });
        return ExchangeRateResponse { result: vec![item] };
    }

    // ERC20 token: without oracle integration, return an error for now
    ExchangeRateResponse {
        result: vec![ExchangeRateResultItem::Error(ExchangeRateError {
            error: ExchangeRateErrorBody {
                id: req.token.clone(),
                message: "token exchange rate unavailable".to_string(),
            },
        })],
    }
}

/// Build a response for the relayer_getStatus endpoint
fn build_get_status_response(_req: &GetStatusRequest) -> GetStatusResponse {
    GetStatusResponse {
		result: vec![StatusResult {
			version: "2.0.0".to_string(),
			id: "0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331".to_string(),
			status: 200,
			receipts: vec![Receipt {
				logs: vec![Log {
					address: "0xa922b54716264130634d6ff183747a8ead91a40b".to_string(),
					topics: vec!["0x5a2a90727cc9d000dd060b1132a5c977c9702bb3a52afe360c9c22f0e9451a68".to_string()],
					data: "0xabcd".to_string(),
				}],
				status: "0x1".to_string(),
				block_hash: "0xf19bbafd9fd0124ec110b848e8de4ab4f62bf60c189524e54213285e7f540d4a".to_string(),
				block_number: "0xabcd".to_string(),
				gas_used: "0xdef".to_string(),
				transaction_hash: "0x9b7bb827c2e5e3c1a0a44dc53e573aa0b3af3bd1f9f5ed03071b100bb039eaff".to_string(),
				chain_id: "1".to_string(),
			}],
			resubmissions: vec![Resubmission {
				status: 200,
				transaction_hash: "0x9b7bb827c2e5e3c1a0a44dc53e573aa0b3af3bd1f9f5ed03071b100bb039eaf3".to_string(),
				chain_id: "1".to_string(),
			}],
			offchain_failure: vec![OffchainFailure {
				message: "insufficient fee provided".to_string(),
			}],
			onchain_failure: vec![OnchainFailure {
				transaction_hash: "0x9b7bb827c2e5e3c1a0a44dc53e573aa0b3af3bd1f9f5ed03071b100bb039eaf2".to_string(),
				chain_id: "1".to_string(),
				message: "execution reverted: transfer failed".to_string(),
				data: "0x08c379a000000000000000000000000000000000000000000000000000000000".to_string(),
			}],
		}],
	}
}

// Build a response for the relayer_sendTransaction endpoint (removed unused builder)
/// Build a response for the relayer_getQuote endpoint
fn build_quote_response() -> QuoteResponse {
    QuoteResponse {
		quote: QuoteInner {
			fee: 132,
			rate: 3702.23,
			token: TokenInfo {
				decimals: 6,
				address: "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string(),
				symbol: Some("USDC".to_string()),
				name: Some("USDC".to_string()),
			},
		},
		relayer_calls: vec![RelayerCall { to: "0x...".to_string(), data: "0x...".to_string() }],
		fee_collector: "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string(),
		revert_reason: "0x87f20438000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000840000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000008408c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002645524332303a207472616e7366657220616d6f756e7420657863656564732062616c616e6365000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_string(),
	}
}

impl RpcServer {
    pub fn new(host: String, port: u16, storage: Storage, config: Config) -> Result<Self> {
        Ok(Self {
            host,
            port,
            storage,
            config,
        })
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Initializing JSON-RPC handler");
        let mut io = IoHandler::new();

        // Endpoint 1: relayer_sendTransaction
        tracing::debug!("Registering endpoint: relayer_sendTransaction");
        let storage1 = self.storage.clone();
        let cfg1 = self.config.clone();
        io.add_method("relayer_sendTransaction", move |params: Params| {
            let storage = storage1.clone();
            let cfg = cfg1.clone();

            async move {
                let inputs: Vec<SendTransactionRequest> = params
                    .parse::<Vec<SendTransactionRequest>>()
                    .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
                let input = inputs.first().ok_or_else(|| {
                    jsonrpc_core::Error::invalid_params("missing params: expected one object")
                })?;

                let response = process_send_transaction(storage, input, &cfg).await?;
                serde_json::to_value(response).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // Endpoint 1b: relayer_sendTransactionMultichain
        tracing::debug!("Registering endpoint: relayer_sendTransactionMultichain");
        let storage1b = self.storage.clone();
        let cfg1b = self.config.clone();
        io.add_method(
            "relayer_sendTransactionMultichain",
            move |params: Params| {
                let storage = storage1b.clone();
                let cfg = cfg1b.clone();

                async move {
                    let inputs: Vec<SendTransactionMultichainRequest> = params
                        .parse::<Vec<SendTransactionMultichainRequest>>()
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
                    let input = inputs.first().ok_or_else(|| {
                        jsonrpc_core::Error::invalid_params("missing params: expected one object")
                    })?;

                    let response =
                        process_send_transaction_multichain(storage, input, &cfg).await?;
                    serde_json::to_value(response)
                        .map_err(|_| jsonrpc_core::Error::internal_error())
                }
            },
        );

        // Endpoint 2: relayer_getStatus
        tracing::debug!("Registering endpoint: relayer_getStatus");
        let storage2 = self.storage.clone();
        let cfg2 = self.config.clone();
        io.add_method("relayer_getStatus", move |params: Params| {
            let storage = storage2.clone();
            let cfg = cfg2.clone();

            async move {
                let request: GetStatusRequest = params
                    .parse::<GetStatusRequest>()
                    .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

                let response = process_get_status(storage, &request, &cfg).await?;
                serde_json::to_value(response).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // Endpoint 3: Health check
        tracing::debug!("Registering endpoint: health_check");
        let storage3 = self.storage.clone();
        let cfg3 = self.config.clone();
        io.add_method("health_check", move |_params: Params| {
            let storage = storage3.clone();
            let cfg = cfg3.clone();

            async move {
                let health = process_health_check(storage, &cfg).await?;

                serde_json::to_value(health).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // New Endpoint: relayer_getExchangeRate
        tracing::debug!("Registering endpoint: relayer_getExchangeRate");
        let cfg4 = self.config.clone();
        io.add_method("relayer_getExchangeRate", move |params: Params| {
            let cfg = cfg4.clone();
            async move {
                let inputs: Vec<ExchangeRateRequest> =
                    params
                        .parse::<Vec<ExchangeRateRequest>>()
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
                let input = inputs.first().ok_or_else(|| {
                    jsonrpc_core::Error::invalid_params("missing params: expected one object")
                })?;

                let payload = build_exchange_rate_response(&cfg, input).await;
                serde_json::to_value(payload).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // New Endpoint: relayer_getQuote
        tracing::debug!("Registering endpoint: relayer_getQuote");
        let cfg6 = self.config.clone();
        io.add_method("relayer_getQuote", move |params: Params| {
            let cfg = cfg6.clone();
            async move {
                let inputs: Vec<QuoteRequest> = params
                    .parse::<Vec<QuoteRequest>>()
                    .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
                let input = inputs.first().ok_or_else(|| jsonrpc_core::Error::invalid_params("missing params: expected one object"))?;

                // Minimal realistic quote: estimate gas and use current gas price
                let chain_id: u64 = input
                    .chain_id
                    .as_ref()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);

                let gas_limit = simulate_transaction(&input.to, &input.data, chain_id, &cfg)
                    .await
                    .unwrap_or(21000);

                let gas_price_hex = fetch_gas_price(chain_id, &cfg)
                    .await
                    .unwrap_or_else(|_| "0x4a817c800".to_string());
                let wei_per_gas = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
                    .unwrap_or(20_000_000_000);
                let fee_wei = (wei_per_gas as u128).saturating_mul(gas_limit as u128);
                let fee = u64::try_from(fee_wei.min(u128::from(u64::MAX))).unwrap_or(u64::MAX);

                let payload = QuoteResponse {
                    quote: QuoteInner {
                        fee,
                        rate: (wei_per_gas as f64) / 1e18_f64,
                        token: TokenInfo {
                            decimals: 18,
                            address: "0x0000000000000000000000000000000000000000".to_string(),
                            symbol: Some("ETH".to_string()),
                            name: Some("Ethereum".to_string()),
                        },
                    },
                    relayer_calls: vec![RelayerCall {
                        to: input.to.clone(),
                        data: input.data.clone(),
                    }],
                    fee_collector: std::env::var("RELAYX_FEE_COLLECTOR").ok().unwrap_or_else(
                        || "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string(),
                    ),
                    revert_reason: "".to_string(),
                };

                serde_json::to_value(payload).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // New Endpoint: relayer_getCapabilities
        tracing::debug!("Registering endpoint: relayer_getCapabilities");
        let storage5 = self.storage.clone();
        let cfg5 = self.config.clone();
        io.add_method("relayer_getCapabilities", move |_params: Params| {
            let storage = storage5.clone();
            let cfg = cfg5.clone();

            async move {
                let capabilities = process_get_capabilities(storage, &cfg).await?;
                serde_json::to_value(capabilities)
                    .map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // Start the HTTP server
        tracing::info!("Starting HTTP server");
        let addr = format!("{}:{}", self.host, self.port);
        let socket_addr: SocketAddr = addr.parse().map_err(|e| {
            tracing::error!("Invalid server address '{}': {}", addr, e);
            anyhow::anyhow!("Invalid address: {}", e)
        })?;

        tracing::debug!("Binding server to address: {}", socket_addr);
        let server = ServerBuilder::new(io)
            .threads(4)
            .start_http(&socket_addr)
            .map_err(|e| {
                tracing::error!("Failed to start HTTP server on {}: {}", socket_addr, e);
                e
            })?;

        tracing::info!("✓ JSON-RPC server listening on {}", socket_addr);
        tracing::info!("Available endpoints:");
        tracing::info!("  - relayer_sendTransaction");
        tracing::info!("  - relayer_sendTransactionMultichain");
        tracing::info!("  - relayer_getStatus");
        tracing::info!("  - relayer_getCapabilities");
        tracing::info!("  - relayer_getExchangeRate");
        tracing::info!("  - relayer_getQuote");
        tracing::info!("  - health_check");

        // Spawn background monitor for pending/processing transactions
        {
            let storage_bg = self.storage.clone();
            let cfg_bg = self.config.clone();
            tokio::spawn(async move {
                loop {
                    // Poll every 10 seconds
                    sleep(Duration::from_secs(10)).await;
                    if let Ok(requests) = storage_bg.get_requests(Some(1000)).await {
                        for req in requests {
                            if matches!(
                                req.status,
                                RequestStatus::Pending | RequestStatus::Processing
                            ) {
                                if let Some(tx_hash) = req.transaction_hash.clone() {
                                    // Try fetch receipt
                                    if let Some(receipt_status) = fetch_and_update_receipt(
                                        &storage_bg,
                                        &cfg_bg,
                                        &req,
                                        &tx_hash,
                                    )
                                    .await
                                    {
                                        tracing::debug!(
                                            "Receipt processed for {} => {:?}",
                                            req.id,
                                            receipt_status
                                        );
                                    } else {
                                        // If still pending, attempt gas-bump resubmission
                                        if let Ok(price_hex) =
                                            fetch_gas_price(req.chain_id, &cfg_bg).await
                                        {
                                            let bumped = bump_gas_price_hex(&price_hex, 20);
                                            if let Some(data) = req.data.clone() {
                                                match send_relay_transaction(
                                                    &req.to_address,
                                                    &data,
                                                    req.chain_id,
                                                    req.gas_limit,
                                                    &bumped,
                                                    &cfg_bg,
                                                )
                                                .await
                                                {
                                                    Ok(new_tx_hash) => {
                                                        let _ = storage_bg
                                                            .update_request_tx_hash(
                                                                req.id,
                                                                new_tx_hash.clone(),
                                                            )
                                                            .await;
                                                        let _ = storage_bg
                                                            .add_resubmission(
                                                                req.id,
                                                                &Resubmission {
                                                                    status: 201,
                                                                    transaction_hash: new_tx_hash,
                                                                    chain_id: req
                                                                        .chain_id
                                                                        .to_string(),
                                                                },
                                                            )
                                                            .await;
                                                        let _ = storage_bg
                                                            .update_request_status(
                                                                req.id,
                                                                RequestStatus::Processing,
                                                                None,
                                                            )
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        let _ = storage_bg
                                                            .update_request_status(
                                                                req.id,
                                                                RequestStatus::Failed,
                                                                Some(e),
                                                            )
                                                            .await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        // Keep the server running
        tracing::info!("Server is ready and waiting for requests");
        server.wait();

        Ok(())
    }
}

/// Fetch transaction receipt and update storage status accordingly
async fn fetch_and_update_receipt(
    storage: &Storage,
    cfg: &Config,
    req: &RelayerRequest,
    tx_hash: &str,
) -> Option<RequestStatus> {
    let rpc_url = match cfg.rpc_url_for_chain(&req.chain_id.to_string()) {
        Some(u) => u,
        None => return None,
    };

    let provider = ProviderBuilder::new().on_http(rpc_url.parse().ok()?);

    // alloy's get_transaction_receipt expects a TxHash; parse hex string
    let hash = match tx_hash.strip_prefix("0x") {
        Some(s) => s,
        None => tx_hash,
    };

    let hash_bytes = hex::decode(hash).ok()?;
    if hash_bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash_bytes);
    let txh = alloy::primitives::B256::from(arr);

    match provider.get_transaction_receipt(txh).await {
        Ok(Some(rcpt)) => {
            // status: true = success, false = fail
            let status_val = rcpt.status();
            if status_val {
                let _ = storage
                    .update_request_status(req.id, RequestStatus::Completed, None)
                    .await;
                Some(RequestStatus::Completed)
            } else {
                let _ = storage
                    .update_request_status(
                        req.id,
                        RequestStatus::Failed,
                        Some("onchain revert".to_string()),
                    )
                    .await;
                Some(RequestStatus::Failed)
            }
        }
        Ok(None) => None, // not yet mined
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::types::{PaymentCapability, SendTransactionCapabilities};

    fn test_config() -> Config {
        Config {
            rpc_host: "127.0.0.1".to_string(),
            rpc_port: 8545,
            db_path: std::path::PathBuf::from("./relayx_db_test"),
            relayers: "".to_string(),
            max_concurrent_requests: 100,
            request_timeout: 30,
            config_path: None,
            http_address: "127.0.0.1".to_string(),
            http_port: 4937,
            http_cors: "*".to_string(),
            log_level: "debug".to_string(),
        }
    }

    async fn test_storage() -> Storage {
        let dir = tempdir().unwrap();
        Storage::new(dir.path()).unwrap()
    }

    #[tokio::test]
    async fn test_get_capabilities_contains_native_and_sponsored() {
        let storage = test_storage().await;
        let cfg = test_config();
        let resp = super::process_get_capabilities(storage, &cfg)
            .await
            .unwrap();
        let mut has_native = false;
        let mut has_sponsored = false;
        for p in resp.capabilities.payment {
            match p {
                Payment::Native(_) => has_native = true,
                Payment::Sponsored(_) => has_sponsored = true,
                _ => {}
            }
        }
        assert!(has_native && has_sponsored);
    }

    #[tokio::test]
    async fn test_health_check_initial_counts() {
        let storage = test_storage().await;
        let cfg = test_config();
        let health = super::process_health_check(storage, &cfg).await.unwrap();
        assert_eq!(health.total_requests, 0);
        assert_eq!(health.pending_requests, 0);
        assert_eq!(health.completed_requests, 0);
        assert_eq!(health.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_get_status_invalid_id_format() {
        let storage = test_storage().await;
        let cfg = test_config();
        let req = GetStatusRequest {
            ids: vec!["not-a-uuid".to_string()],
        };
        let resp = super::process_get_status(storage, &req, &cfg).await.unwrap();
        assert_eq!(resp.result.len(), 1);
        assert_eq!(resp.result[0].status, 400);
    }

    #[tokio::test]
    async fn test_send_transaction_missing_fields() {
        let storage = test_storage().await;
        let cfg = test_config();

        // Missing 'to'
        let req1 = SendTransactionRequest {
            to: "".to_string(),
            data: "0x".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: "".to_string(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: "".to_string(),
        };
        let err = super::process_send_transaction(storage.clone(), &req1, &cfg).await.err().unwrap();
        assert_eq!(err.code, jsonrpc_core::ErrorCode::InvalidParams);

        // Missing 'data'
        let req2 = SendTransactionRequest {
            data: "".to_string(),
            ..req1.clone()
        };
        let err = super::process_send_transaction(storage.clone(), &req2, &cfg).await.err().unwrap();
        assert_eq!(err.code, jsonrpc_core::ErrorCode::InvalidParams);

        // Missing 'chainId'
        let req3 = SendTransactionRequest {
            chain_id: "".to_string(),
            data: "0x12".to_string(),
            ..req1.clone()
        };
        let err = super::process_send_transaction(storage.clone(), &req3, &cfg).await.err().unwrap();
        assert_eq!(err.code, jsonrpc_core::ErrorCode::InvalidParams);
    }

    #[tokio::test]
    async fn test_send_transaction_unsupported_chain() {
        let storage = test_storage().await;
        let cfg = test_config();
        let req = SendTransactionRequest {
            to: "0x0000000000000000000000000000000000000000".to_string(),
            data: "0x12".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: "".to_string(),
                },
            },
            chain_id: "999999".to_string(),
            authorization_list: "".to_string(),
        };
        let err = super::process_send_transaction(storage, &req, &cfg).await.err().unwrap();
        assert_eq!(err.code, jsonrpc_core::ErrorCode::InvalidParams);
    }

    #[tokio::test]
    async fn test_multichain_empty_transactions() {
        let storage = test_storage().await;
        let cfg = test_config();
        let req = SendTransactionMultichainRequest {
            transactions: vec![],
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: "".to_string(),
                },
            },
            payment_chain_id: "1".to_string(),
        };
        let err = super::process_send_transaction_multichain(storage, &req, &cfg).await.err().unwrap();
        assert_eq!(err.code, jsonrpc_core::ErrorCode::InvalidParams);
    }

    #[tokio::test]
    async fn test_exchange_rate_invalid_chain_and_erc20_unavailable() {
        let cfg = test_config();
        // invalid chain id
        let r1 = ExchangeRateRequest {
            token: "0x0000000000000000000000000000000000000000".to_string(),
            chain_id: "abc".to_string(),
        };
        let resp1 = super::build_exchange_rate_response(&cfg, &r1).await;
        assert!(matches!(
            resp1.result.first().unwrap(),
            ExchangeRateResultItem::Error(_)
        ));

        // ERC20 unavailable
        let r2 = ExchangeRateRequest {
            token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            chain_id: "1".to_string(),
        };
        let resp2 = super::build_exchange_rate_response(&cfg, &r2).await;
        assert!(matches!(
            resp2.result.first().unwrap(),
            ExchangeRateResultItem::Error(_)
        ));
    }
}
