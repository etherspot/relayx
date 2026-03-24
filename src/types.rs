use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== Internal storage types (unchanged) =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerRequest {
    pub id: Uuid,
    pub task_id: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub gas_limit: u64,
    pub gas_price: String,
    pub data: Option<String>,
    pub nonce: u64,
    pub chain_id: u64,
    pub transaction_hash: Option<String>,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
    /// Optional webhook URL to POST the final status to when the transaction settles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestQuery {
    pub status: Option<RequestStatus>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub chain_id: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

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

// ===== Shared spec types =====

/// Token descriptor used in both capabilities and fee data responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDetails {
    pub address: String,
    pub decimals: u8,
}

/// Resubmission record (internal, returned inside legacy getStatus).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resubmission {
    pub status: u16,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

// ===== relayer_sendTransaction / relayer_sendTransactionMultichain =====

/// Payment object per the spec: type is "token" (covers native and ERC20 by address)
/// or "sponsored" for gas-sponsored transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    #[serde(rename = "type")]
    pub payment_type: String,
    /// Token address (zero address for native ETH). Required when type == "token".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Arbitrary relayer data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// EIP-7702 authorization entry in JSON format (per spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationItem {
    pub address: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    pub nonce: u64,
    pub r: String,
    pub s: String,
    #[serde(rename = "yParity")]
    pub y_parity: u8,
}

/// Params for relayer_sendTransaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionParams {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub payment: Payment,
    /// Target wallet address (smart account) to execute the transaction on.
    pub to: String,
    /// Encoded executeWithRelayer calldata.
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(rename = "authorizationList", skip_serializing_if = "Option::is_none")]
    pub authorization_list: Option<Vec<AuthorizationItem>>,
    /// Optional client-provided task ID (32-byte hex string, 0x-prefixed).
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

// ===== relayer_getStatus =====

/// Params for relayer_getStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusParams {
    pub id: String,
    pub logs: bool,
}

/// Log entry returned in a confirmed receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

/// Receipt returned for a confirmed (200) status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecReceipt {
    #[serde(rename = "blockHash")]
    pub block_hash: String,
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<Log>>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
}

/// Spec-defined status codes.
///
/// - 100: Pending (received, not yet submitted on-chain)
/// - 110: Submitted (on-chain, awaiting confirmation)
/// - 200: Confirmed (successfully included)
/// - 400: Rejected (off-chain failure)
/// - 500: Reverted (on-chain failure)
///
/// Flat struct with optional fields; which fields are present depends on the status code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecStatusResponse {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    pub status: u16,
    /// On-chain tx hash; present for status 110 (submitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Full receipt; present for status 200 (confirmed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<SpecReceipt>,
    /// Human-readable error description; present for status 400 and 500.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Revert data (hex string); present for status 500. May also be present for 400.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ===== relayer_getCapabilities =====

/// Per-chain capability info returned by relayer_getCapabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCapabilities {
    #[serde(rename = "feeCollector")]
    pub fee_collector: String,
    pub tokens: Vec<TokenDetails>,
}

/// Response is a map of chain ID -> capabilities.
pub type GetCapabilitiesResponse = HashMap<String, ChainCapabilities>;

// ===== relayer_getFeeData =====

/// Params for relayer_getFeeData.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDataParams {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub token: String,
}

/// Response for relayer_getFeeData.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDataResponse {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub token: TokenDetails,
    /// Tokens per 1 unit of native currency (e.g., USDC/ETH = 2000.5).
    /// For native token payments this is always 1.0.
    pub rate: f64,
    /// Minimum fee denominated in token units (human-readable), if applicable.
    #[serde(rename = "minFee", skip_serializing_if = "Option::is_none")]
    pub min_fee: Option<String>,
    /// Unix timestamp when this quote expires.
    pub expiry: u64,
    /// Legacy gas price in wei (hex). Clients SHOULD prefer maxFeePerGas when present.
    #[serde(rename = "gasPrice")]
    pub gas_price: String,
    /// EIP-1559 max fee per gas in wei (hex). Present when the chain supports EIP-1559.
    #[serde(rename = "maxFeePerGas", skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<String>,
    /// EIP-1559 max priority fee per gas in wei (hex). Present alongside maxFeePerGas.
    #[serde(
        rename = "maxPriorityFeePerGas",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_priority_fee_per_gas: Option<String>,
    /// Fee collector address clients should transfer payment tokens to.
    /// Duplicates relayer_getCapabilities for single-call convenience.
    #[serde(rename = "feeCollector", skip_serializing_if = "Option::is_none")]
    pub fee_collector: Option<String>,
    /// Opaque context for the relayer (e.g., signed quote).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

// ===== relayer_getQuote (non-spec, retained for backward compat) =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub decimals: u8,
    pub address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerCall {
    pub to: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequestCapabilities {
    #[serde(default)]
    pub payment: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub to: String,
    pub data: String,
    #[serde(default)]
    pub capabilities: Option<QuoteRequestCapabilities>,
    #[serde(rename = "chainId")]
    pub chain_id: Option<String>,
    #[serde(rename = "authorizationList")]
    pub authorization_list: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteInner {
    pub fee: u64,
    pub rate: f64,
    pub token: TokenInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub quote: QuoteInner,
    #[serde(rename = "relayerCalls")]
    pub relayer_calls: Vec<RelayerCall>,
    #[serde(rename = "feeCollector")]
    pub fee_collector: String,
    #[serde(rename = "revertReason")]
    pub revert_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasFees {
    /// Legacy / base gas price (hex wei). Always present.
    pub gas_price: String,
    /// EIP-1559 max fee per gas (hex wei). None on pre-EIP-1559 chains.
    pub max_fee_per_gas: Option<String>,
    /// EIP-1559 max priority fee per gas (hex wei). None on pre-EIP-1559 chains.
    pub max_priority_fee_per_gas: Option<String>,
}
