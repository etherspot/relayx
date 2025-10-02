use std::net::SocketAddr;
use anyhow::Result;
use chrono::Utc;
use jsonrpc_core::{IoHandler, Params};
use jsonrpc_http_server::ServerBuilder;
use uuid::Uuid;

use crate::{
    config::Config,
    storage::Storage,
    types::{
        Capabilities, Erc20Payment, ExchangeRateRequest, ExchangeRateResponse, ExchangeRateResultItem, 
        ExchangeRateSuccess, ExchangeRateQuote, GetCapabilitiesResponse, GetStatusRequest, GetStatusResponse, HealthResponse, Log, 
        NativePayment, OffchainFailure, OnchainFailure, Payment, PaymentType, QuoteInner,
        QuoteRequest, QuoteResponse, Receipt, RelayerCall, RequestStatus, Resubmission,
        SendTransactionRequest, SendTransactionResponse, SendTransactionResult, SponsoredPayment,
        StatusResult, TokenInfo,
    },
};

pub struct RpcServer {
    host: String,
    port: u16,
    storage: Storage,
    config: Config,
}

/// Endpoint business logic functions
async fn process_send_transaction(
    _storage: Storage,
    _input: &SendTransactionRequest,
    _cfg: &Config,
) -> Result<SendTransactionResponse, jsonrpc_core::Error> {
    // For now, return stub result as per existing stubbed builder
    Ok(SendTransactionResponse {
        result: vec![SendTransactionResult {
            chain_id: "1".into(),
            id: String::new(),
        }],
    })
}

async fn process_get_status(
    storage: Storage,
    request: &GetStatusRequest,
    _cfg: &Config,
) -> Result<GetStatusResponse, jsonrpc_core::Error> {
    for id in &request.ids {
        if let Ok(uuid) = Uuid::parse_str(id) {
            if let Ok(Some(_req)) = storage.get_request(uuid).await {
                // Could enrich response using stored data
            }
        }
    }
    Ok(build_get_status_response(request))
}

async fn process_health_check(
    storage: Storage,
    _cfg: &Config,
) -> Result<HealthResponse, jsonrpc_core::Error> {
    let total_requests = storage
        .get_total_request_count()
        .await
        .map_err(|_| jsonrpc_core::Error::internal_error())?;

    let pending_requests = storage
        .get_request_count_by_status(RequestStatus::Pending)
        .await
        .map_err(|_| jsonrpc_core::Error::internal_error())?;

    let completed_requests = storage
        .get_request_count_by_status(RequestStatus::Completed)
        .await
        .map_err(|_| jsonrpc_core::Error::internal_error())?;

    let failed_requests = storage
        .get_request_count_by_status(RequestStatus::Failed)
        .await
        .map_err(|_| jsonrpc_core::Error::internal_error())?;

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
    // Build capabilities based on configuration
    // Extract all supported tokens from the chainlink configuration
    let supported_tokens = cfg.get_supported_tokens();
    
    let mut payments = Vec::new();
    
    // Add ERC20 payment options for each supported token
    for token in supported_tokens {
        payments.push(Payment::Erc20(Erc20Payment {
            payment_type: PaymentType::Erc20,
            token,
        }));
    }
    
    // If no tokens found in config, fall back to default token
    if payments.is_empty() {
        let default_token = cfg.default_token()
            .unwrap_or_else(|| "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string()); // USDC on Ethereum
        
        payments.push(Payment::Erc20(Erc20Payment {
            payment_type: PaymentType::Erc20,
            token: default_token,
        }));
    }
    
    // Always include native payment option
    payments.push(Payment::Native(NativePayment {
        payment_type: PaymentType::Native,
        token: "0x0000000000000000000000000000000000000000".to_string(),
    }));
    
    // Always include sponsored payment option
    payments.push(Payment::Sponsored(SponsoredPayment {
        payment_type: PaymentType::Sponsored,
    }));

    let capabilities = Capabilities { payment: payments };

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

/// Build a simple stub response for the relayer_getExchangeRate endpoint
async fn build_exchange_rate_response_stub(
    cfg: &Config,
    req: &ExchangeRateRequest,
) -> ExchangeRateResponse {
    let now = Utc::now().timestamp() as u64;
    let expiry = now + 600;
    
    // Zero address denotes native token
    let zero_addr = "0x0000000000000000000000000000000000000000".to_string();
    
    let result_item = if req.token.to_lowercase() == zero_addr {
        // Native token: return a simple rate
        ExchangeRateResultItem::Success(ExchangeRateSuccess {
            quote: ExchangeRateQuote {
                rate: 0.001, // 0.001 ETH per gas
                token: TokenInfo {
                    decimals: 18,
                    address: zero_addr.clone(),
                    symbol: Some("ETH".to_string()),
                    name: Some("Ethereum".to_string()),
                },
            },
            gas_price: "0x4a817c800".to_string(), // 20 gwei
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            fee_collector: std::env::var("RELAYX_FEE_COLLECTOR")
                .ok()
                .or_else(|| cfg.fee_collector())
                .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string()),
            expiry,
        })
    } else {
        // ERC20 token: return a simple rate
        ExchangeRateResultItem::Success(ExchangeRateSuccess {
            quote: ExchangeRateQuote {
                rate: 0.0032, // Example rate for USDC
                token: TokenInfo {
                    decimals: 6,
                    address: req.token.clone(),
                    symbol: Some("USDC".to_string()),
                    name: Some("USD Coin".to_string()),
                },
            },
            gas_price: "0x4a817c800".to_string(), // 20 gwei
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            fee_collector: std::env::var("RELAYX_FEE_COLLECTOR")
                .ok()
                .or_else(|| cfg.fee_collector())
                .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string()),
            expiry,
        })
    };

    ExchangeRateResponse {
        result: vec![result_item],
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
        let mut io = IoHandler::new();

        // Endpoint 1: relayer_sendTransaction
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

        // Endpoint 2: relayer_getStatus
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

                let payload = build_exchange_rate_response_stub(&cfg, input).await;
                serde_json::to_value(payload).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // New Endpoint: relayer_getQuote
        io.add_method("relayer_getQuote", |params: Params| async move {
            let _inputs: Vec<QuoteRequest> = params
                .parse::<Vec<QuoteRequest>>()
                .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
            let payload = build_quote_response();
            serde_json::to_value(payload).map_err(|_| jsonrpc_core::Error::internal_error())
        });

        // New Endpoint: relayer_getCapabilities
        let storage5 = self.storage.clone();
        let cfg5 = self.config.clone();
        io.add_method("relayer_getCapabilities", move |_params: Params| {
            let storage = storage5.clone();
            let cfg = cfg5.clone();

            async move {
                let capabilities = process_get_capabilities(storage, &cfg).await?;
                serde_json::to_value(capabilities).map_err(|_| jsonrpc_core::Error::internal_error())
            }
        });

        // Start the HTTP server
        let addr = format!("{}:{}", self.host, self.port);
        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        let server = ServerBuilder::new(io).threads(4).start_http(&socket_addr)?;

        tracing::info!("JSON-RPC server started on {}", socket_addr);

        // Keep the server running
        server.wait();

        Ok(())
    }
}
