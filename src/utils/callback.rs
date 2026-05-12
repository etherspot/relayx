//! Status update webhooks for wallet integrations.
//!
//! Outbound `POST` bodies follow the OKX Wallet “Submit Intent Status” shape
//! ([Relayer Integration API](https://hackmd.io/@michaelwong/SJVSZ1wcgx)): a **JSON array**
//! (max 100 items per request in that spec; we send one item per callback) of flattened
//! status objects using `chainIndex`, `requestId`, `txHash`, etc.

use serde::Serialize;
use uuid::Uuid;

use crate::{config::Config, RelayerRequest, SpecStatusResponse};

/// Convert a decimal quantity string (e.g. block number from receipts) to a `0x` hex string
/// as required by the OKX transaction-status webhook for `blockHeight` / `gasUsed`.
fn decimal_string_to_hex_quantity(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        if rest.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(format!("0x{}", rest.to_ascii_lowercase()));
        }
        return None;
    }
    let n = u128::from_str_radix(t, 10).ok()?;
    Some(format!("0x{:x}", n))
}

fn error_data_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}

/// One row in the OKX `transaction-status` webhook array.
#[derive(Debug, Serialize)]
pub struct OkxTransactionStatusItem {
    /// Unix timestamp in seconds when this status push was generated (string).
    pub timestamp: String,
    /// Unique identifier for this status update delivery.
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// OKX chain index; defaults to EIP-155 `chainId` unless overridden in config.
    #[serde(rename = "chainIndex")]
    pub chain_index: String,
    /// EIP relayer status code as a string (`"100"`, `"110"`, …).
    pub status: String,
    #[serde(rename = "blockHash", skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    #[serde(rename = "blockHeight", skip_serializing_if = "Option::is_none")]
    pub block_height: Option<String>,
    #[serde(rename = "gasUsed", skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<String>,
    #[serde(rename = "txHash", skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(rename = "errorData", skip_serializing_if = "Option::is_none")]
    pub error_data: Option<String>,
}

/// Build a single OKX-format status item from internal relayer state.
pub fn build_okx_transaction_status_item(
    req: &RelayerRequest,
    status: &SpecStatusResponse,
    cfg: &Config,
) -> OkxTransactionStatusItem {
    let chain_index = cfg.chain_index_for_chain_id(&req.chain_id.to_string());
    let status_str = status.status.to_string();

    let mut item = OkxTransactionStatusItem {
        timestamp: chrono::Utc::now().timestamp().to_string(),
        request_id: Uuid::new_v4().to_string(),
        task_id: req.task_id.clone(),
        chain_index,
        status: status_str,
        block_hash: None,
        block_height: None,
        gas_used: None,
        tx_hash: None,
        error_message: None,
        error_data: None,
    };

    match status.status {
        110 => {
            item.tx_hash = status.hash.clone().or_else(|| req.transaction_hash.clone());
        }
        200 => {
            if let Some(ref r) = status.receipt {
                item.block_hash = Some(r.block_hash.clone());
                item.block_height = decimal_string_to_hex_quantity(&r.block_number);
                item.gas_used = decimal_string_to_hex_quantity(&r.gas_used);
                item.tx_hash = Some(r.transaction_hash.clone());
            } else {
                item.tx_hash = status.hash.clone().or_else(|| req.transaction_hash.clone());
            }
        }
        400 | 500 => {
            item.error_message = Some(status.message.clone().unwrap_or_default());
            item.error_data = status.data.as_ref().map(|d| error_data_to_string(d));
        }
        _ => {}
    }

    item
}

/// POST the status update to the callback URL as a **JSON array** of
/// [`OkxTransactionStatusItem`] (OKX “Submit Intent Status” wire format).
///
/// Failures are logged and silently swallowed — a failed callback never affects the relay flow.
pub async fn fire_callback(req: &RelayerRequest, status: &SpecStatusResponse, cfg: &Config) {
    let url = match &req.callback_url {
        Some(u) if !u.is_empty() => u.clone(),
        _ => return,
    };

    let item = build_okx_transaction_status_item(req, status, cfg);
    let payload = vec![item];

    match reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await
    {
        Ok(resp) => {
            tracing::info!(
                "Callback delivered for task_id {} → {} (HTTP {})",
                req.task_id,
                url,
                resp.status()
            );
        }
        Err(e) => {
            tracing::warn!(
                "Callback failed for task_id {} → {}: {}",
                req.task_id,
                url,
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RequestStatus, SpecReceipt};
    use chrono::Utc;
    use uuid::Uuid;

    fn minimal_cfg() -> Config {
        Config {
            rpc_host: "127.0.0.1".to_string(),
            rpc_port: 8545,
            db_path: std::path::PathBuf::from("./relayx_db_cb_test"),
            relayers: String::new(),
            max_concurrent_requests: 10,
            request_timeout: 30,
            config_path: None,
            http_address: "127.0.0.1".to_string(),
            http_port: 4937,
            http_cors: "*".to_string(),
            log_level: "error".to_string(),
            relayer_private_key: None,
            disable_simulation: false,
            sentry_dsn: None,
            disable_multichain: false,
        }
    }

    fn sample_req() -> RelayerRequest {
        RelayerRequest {
            id: Uuid::new_v4(),
            task_id: "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"
                .to_string(),
            from_address: "0x1".to_string(),
            to_address: "0x2".to_string(),
            amount: "0".to_string(),
            gas_limit: 21000,
            gas_price: "0x1".to_string(),
            data: None,
            nonce: 0,
            chain_id: 1,
            transaction_hash: Some(
                "0xd9b01a72502e7f518fb043bfacd1e13b07f24995f404f8cbb60a1212ca8b4c42".to_string(),
            ),
            status: RequestStatus::Completed,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
            callback_url: Some("https://example.invalid/webhook".to_string()),
        }
    }

    #[test]
    fn okx_payload_is_array_with_expected_keys_for_status_200() {
        let cfg = minimal_cfg();
        let req = sample_req();
        let status = SpecStatusResponse {
            chain_id: "1".to_string(),
            created_at: 1755917874,
            status: 200,
            hash: None,
            receipt: Some(SpecReceipt {
                block_hash: "0x6789b0746d84002f2f258129cfd9714d412e78b4d91b8e61608fac9165988baf"
                    .to_string(),
                block_number: "36314734".to_string(),
                gas_used: "40178".to_string(),
                logs: None,
                transaction_hash:
                    "0xd9b01a72502e7f518fb043bfacd1e13b07f24995f404f8cbb60a1212ca8b4c42".to_string(),
            }),
            message: None,
            data: None,
        };

        let item = build_okx_transaction_status_item(&req, &status, &cfg);
        let json = serde_json::to_value(&vec![item]).unwrap();
        assert!(json.is_array());
        let row = &json[0];
        assert_eq!(row["chainIndex"], "1");
        assert_eq!(row["status"], "200");
        assert_eq!(
            row["taskId"],
            "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"
        );
        assert_eq!(
            row["blockHash"],
            "0x6789b0746d84002f2f258129cfd9714d412e78b4d91b8e61608fac9165988baf"
        );
        assert_eq!(row["blockHeight"], "0x22a1e6e");
        assert_eq!(row["gasUsed"], "0x9cf2");
        assert_eq!(
            row["txHash"],
            "0xd9b01a72502e7f518fb043bfacd1e13b07f24995f404f8cbb60a1212ca8b4c42"
        );
        assert!(row.get("errorMessage").is_none());
    }

    #[test]
    fn okx_payload_for_status_400_includes_error_fields() {
        let cfg = minimal_cfg();
        let req = sample_req();
        let status = SpecStatusResponse {
            chain_id: "1".to_string(),
            created_at: 1,
            status: 400,
            hash: None,
            receipt: None,
            message: Some("Insufficient payment".to_string()),
            data: Some(serde_json::json!("0x")),
        };

        let item = build_okx_transaction_status_item(&req, &status, &cfg);
        let v = serde_json::to_value(&item).unwrap();
        assert_eq!(v["status"], "400");
        assert_eq!(v["errorMessage"], "Insufficient payment");
        assert_eq!(v["errorData"], "0x");
    }
}
