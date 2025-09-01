use crate::storage::Storage;
use crate::types::{
    HealthResponse, NewRequestInput, RelayerRequest, RelayerResponse, RequestStatus,
};
use anyhow::Result;
use chrono::Utc;
use jsonrpc_core::{IoHandler, Params};
use jsonrpc_http_server::ServerBuilder;
use serde_json::Value;
use std::net::SocketAddr;
use uuid::Uuid;

pub struct RpcServer {
    host: String,
    port: u16,
    storage: Storage,
}

impl RpcServer {
    pub fn new(host: String, port: u16, storage: Storage) -> Result<Self> {
        Ok(Self {
            host,
            port,
            storage,
        })
    }

    pub async fn start(&self) -> Result<()> {
        let mut io = IoHandler::new();

        // Endpoint 1: Submit a new relayer request
        let storage1 = self.storage.clone();
        io.add_method("submit_request", move |params: Params| {
            let storage = storage1.clone();

            async move {
                let params = params
                    .parse::<NewRequestInput>()
                    .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

                let request = RelayerRequest {
                    id: Uuid::new_v4(),
                    from_address: params.from_address,
                    to_address: params.to_address,
                    amount: params.amount,
                    gas_limit: params.gas_limit,
                    gas_price: params.gas_price,
                    data: params.data,
                    nonce: params.nonce,
                    chain_id: params.chain_id,
                    status: RequestStatus::Pending,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    error_message: None,
                };

                storage
                    .store_request(&request)
                    .await
                    .map_err(|_| jsonrpc_core::Error::internal_error())?;

                Ok(Value::String(request.id.to_string()))
            }
        });

        // Endpoint 2: Get request status
        let storage2 = self.storage.clone();
        io.add_method("get_request_status", move |params: Params| {
            let storage = storage2.clone();

            async move {
                let request_id: String = params
                    .parse::<String>()
                    .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

                let uuid = Uuid::parse_str(&request_id)
                    .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

                let request = storage
                    .get_request(uuid)
                    .await
                    .map_err(|_| jsonrpc_core::Error::internal_error())?;

                match request {
                    Some(req) => {
                        let response = RelayerResponse {
                            request_id: req.id,
                            transaction_hash: None, // Would be filled when transaction is processed
                            block_number: None,
                            gas_used: None,
                            status: req.status,
                            completed_at: None,
                            error_message: req.error_message,
                        };

                        Ok(serde_json::to_value(response)
                            .map_err(|_| jsonrpc_core::Error::internal_error())?)
                    }
                    None => Err(jsonrpc_core::Error::invalid_params("Request not found")),
                }
            }
        });

        // Endpoint 3: Health check
        let storage3 = self.storage.clone();
        io.add_method("health_check", move |_params: Params| {
            let storage = storage3.clone();

            async move {
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

                let health = HealthResponse {
                    status: "healthy".to_string(),
                    timestamp: Utc::now(),
                    uptime_seconds: storage.get_uptime_seconds(),
                    total_requests,
                    pending_requests,
                    completed_requests,
                    failed_requests,
                };

                serde_json::to_value(health).map_err(|_| jsonrpc_core::Error::internal_error())
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
