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
    pub transaction_hash: Option<String>,
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

// ===== New endpoint shared types =====

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

// ===== relayer_sendTransaction =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentCapability {
    #[serde(rename = "type")]
    pub payment_type: String,
    pub token: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionCapabilities {
    pub payment: PaymentCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionRequest {
    pub to: String,
    pub data: String,
    pub capabilities: SendTransactionCapabilities,
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "authorizationList")]
    pub authorization_list: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionResult {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionResponse {
    pub result: Vec<SendTransactionResult>,
}

// ===== relayer_sendTransactionMultichain =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultichainTransaction {
    pub to: String,
    pub data: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "authorizationList")]
    pub authorization_list: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionMultichainRequest {
    pub transactions: Vec<MultichainTransaction>,
    pub capabilities: SendTransactionCapabilities,
    #[serde(rename = "paymentChainId")]
    pub payment_chain_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultichainTransactionResult {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionMultichainResponse {
    pub result: Vec<MultichainTransactionResult>,
}

// ===== relayer_getStatus =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusRequest {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub logs: Vec<Log>,
    pub status: String,
    #[serde(rename = "blockHash")]
    pub block_hash: String,
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resubmission {
    pub status: u16,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffchainFailure {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainFailure {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub message: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    pub version: String,
    pub id: String,
    pub status: u16,
    pub receipts: Vec<Receipt>,
    pub resubmissions: Vec<Resubmission>,
    #[serde(rename = "offchainFailure")]
    pub offchain_failure: Vec<OffchainFailure>,
    #[serde(rename = "onchainFailure")]
    pub onchain_failure: Vec<OnchainFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusResponse {
    pub result: Vec<StatusResult>,
}

// ===== relayer_getExchangeRate =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateRequest {
    pub token: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateQuote {
    pub rate: f64, // for 1 unit of gas in token's decimals
    pub token: TokenInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateSuccess {
    pub quote: ExchangeRateQuote,
    #[serde(rename = "gasPrice")]
    pub gas_price: String,
    #[serde(rename = "maxFeePerGas")]
    pub max_fee_per_gas: Option<String>,
    #[serde(rename = "maxPriorityFeePerGas")]
    pub max_priority_fee_per_gas: Option<String>,
    #[serde(rename = "feeCollector")]
    pub fee_collector: String,
    pub expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateErrorBody {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateError {
    pub error: ExchangeRateErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExchangeRateResultItem {
    Success(ExchangeRateSuccess),
    Error(ExchangeRateError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateResponse {
    pub result: Vec<ExchangeRateResultItem>,
}

// ===== relayer_getQuote =====

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

// ===== relayer_getCapabilities =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PaymentType {
    Native,
    #[serde(rename = "erc20")]
    Erc20,
    Sponsored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativePayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub token: String, // "0x0000000000000000000000000000000000000000"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc20Payment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsoredPayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payment {
    Native(NativePayment),
    Erc20(Erc20Payment),
    Sponsored(SponsoredPayment),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub payment: Vec<Payment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCapabilitiesResponse {
    pub capabilities: Capabilities,
}
