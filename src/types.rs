use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Relayer request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerRequest {
    pub id: Uuid,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub gas_limit: u64,
    pub gas_price: String,
    pub data: Option<String>,
    pub nonce: u64,
    pub chain_id: u64,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

/// Relayer response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerResponse {
    pub request_id: Uuid,
    pub transaction_hash: Option<String>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u64>,
    pub status: RequestStatus,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// New request input structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRequestInput {
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub gas_limit: u64,
    pub gas_price: String,
    pub data: Option<String>,
    pub nonce: u64,
    pub chain_id: u64,
}

/// Request query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestQuery {
    pub status: Option<RequestStatus>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub chain_id: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub pending_requests: u64,
    pub completed_requests: u64,
    pub failed_requests: u64,
}
