use alloy::providers::{Provider, ProviderBuilder};
use url::Url;

use crate::{
    utils::{callback::fire_callback, misc::stub_mode_enabled},
    Config, GasFees, Log, RelayerRequest, RequestStatus, SpecReceipt, SpecStatusResponse, Storage,
};

/// Fetch a receipt on-demand for use in getStatus responses.
pub async fn fetch_receipt_for_status(
    tx_hash: &str,
    chain_id: u64,
    cfg: &Config,
) -> Option<SpecReceipt> {
    if stub_mode_enabled() {
        return None;
    }

    let rpc_url = cfg.rpc_url_for_chain(&chain_id.to_string())?;
    let provider = ProviderBuilder::new().on_hyper_http(Url::parse(&rpc_url).ok()?);

    let hash_hex = tx_hash.strip_prefix("0x").unwrap_or(tx_hash);
    let hash_bytes = hex::decode(hash_hex).ok()?;
    if hash_bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash_bytes);
    let txh = alloy::primitives::B256::from(arr);

    match provider.get_transaction_receipt(txh).await {
        Ok(Some(r)) => Some(SpecReceipt {
            block_hash: format!("0x{:x}", r.block_hash.unwrap_or_default()),
            block_number: r.block_number.unwrap_or_default().to_string(),
            gas_used: r.gas_used.to_string(),
            logs: Some(
                r.inner
                    .logs()
                    .iter()
                    .map(|l| Log {
                        address: format!("0x{:x}", l.address()),
                        topics: l.topics().iter().map(|t| format!("0x{:x}", t)).collect(),
                        data: format!("0x{}", hex::encode(l.data().data.as_ref())),
                    })
                    .collect(),
            ),
            transaction_hash: format!("0x{:x}", r.transaction_hash),
        }),
        _ => None,
    }
}

pub async fn fetch_gas_price(chain_id: u64, cfg: &Config) -> Result<String, String> {
    fetch_gas_fees(chain_id, cfg).await.map(|f| f.gas_price)
}

/// Fetch both legacy and EIP-1559 fee data in a single provider call batch.
/// Falls back to stub values in stub mode or on network errors.
pub async fn fetch_gas_fees(chain_id: u64, cfg: &Config) -> Result<GasFees, String> {
    if stub_mode_enabled() {
        return Ok(GasFees {
            gas_price: "0x4a817c800".to_string(),
            max_fee_per_gas: Some("0x77359400".to_string()), // 2 gwei
            max_priority_fee_per_gas: Some("0x3b9aca00".to_string()), // 1 gwei
        });
    }

    let rpc_url = cfg
        .rpc_url_for_chain(&chain_id.to_string())
        .ok_or_else(|| format!("No RPC URL configured for chain {}", chain_id))?;

    let rpc_endpoint = Url::parse(&rpc_url).map_err(|e| format!("Invalid RPC URL: {}", e))?;
    let provider = ProviderBuilder::new().on_hyper_http(rpc_endpoint);

    let gas_price = match provider.get_gas_price().await {
        Ok(p) => format!("0x{:x}", p),
        Err(e) => {
            tracing::warn!("Failed to fetch gas price for chain {}: {}", chain_id, e);
            "0x4a817c800".to_string()
        }
    };

    let (max_fee, max_priority) = match provider.get_fee_history(1, Default::default(), &[]).await {
        Ok(history) if !history.base_fee_per_gas.is_empty() => {
            let base = history.base_fee_per_gas[0];
            // Priority fee = 1 gwei default; max_fee = 2× base + priority (EIP-1559 convention).
            let priority: u128 = 1_000_000_000;
            let max = base.saturating_mul(2) + priority;
            (
                Some(format!("0x{:x}", max)),
                Some(format!("0x{:x}", priority)),
            )
        }
        _ => (None, None),
    };

    Ok(GasFees {
        gas_price,
        max_fee_per_gas: max_fee,
        max_priority_fee_per_gas: max_priority,
    })
}

pub async fn fetch_and_store_receipt(
    storage: &Storage,
    cfg: &Config,
    req: &RelayerRequest,
    tx_hash: &str,
) -> Option<SpecReceipt> {
    if stub_mode_enabled() {
        let _ = storage
            .update_request_status(req.id, RequestStatus::Completed, None)
            .await;
        return None;
    }

    let rpc_url = cfg.rpc_url_for_chain(&req.chain_id.to_string())?;
    let provider = ProviderBuilder::new().on_hyper_http(Url::parse(&rpc_url).ok()?);

    let hash_hex = tx_hash.strip_prefix("0x").unwrap_or(tx_hash);
    let hash_bytes = hex::decode(hash_hex).ok()?;
    if hash_bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash_bytes);
    let txh = alloy::primitives::B256::from(arr);

    match provider.get_transaction_receipt(txh).await {
        Ok(Some(r)) => {
            let status_ok = r.status();
            let receipt = SpecReceipt {
                block_hash: format!("0x{:x}", r.block_hash.unwrap_or_default()),
                block_number: r.block_number.unwrap_or_default().to_string(),
                gas_used: r.gas_used.to_string(),
                logs: Some(
                    r.inner
                        .logs()
                        .iter()
                        .map(|l| Log {
                            address: format!("0x{:x}", l.address()),
                            topics: l.topics().iter().map(|t| format!("0x{:x}", t)).collect(),
                            data: format!("0x{}", hex::encode(l.data().data.as_ref())),
                        })
                        .collect(),
                ),
                transaction_hash: format!("0x{:x}", r.transaction_hash),
            };

            if status_ok {
                let _ = storage.store_receipt(req.id, &receipt).await;
                let _ = storage
                    .update_request_status(req.id, RequestStatus::Completed, None)
                    .await;
                let status_resp = SpecStatusResponse {
                    chain_id: req.chain_id.to_string(),
                    created_at: req.created_at.timestamp() as u64,
                    status: 200,
                    hash: Some(receipt.transaction_hash.clone()),
                    receipt: Some(receipt.clone()),
                    message: None,
                    data: None,
                };
                fire_callback(req, &status_resp, cfg).await;
                Some(receipt)
            } else {
                let msg = "onchain revert".to_string();
                let _ = storage
                    .update_request_status(req.id, RequestStatus::Failed, Some(msg.clone()))
                    .await;
                let status_resp = SpecStatusResponse {
                    chain_id: req.chain_id.to_string(),
                    created_at: req.created_at.timestamp() as u64,
                    status: 500,
                    hash: req.transaction_hash.clone(),
                    receipt: None,
                    message: Some(msg),
                    data: None,
                };
                fire_callback(req, &status_resp, cfg).await;
                None
            }
        }
        Ok(None) => None,
        Err(e) => {
            tracing::warn!("Failed to fetch receipt for {}: {}", req.id, e);
            None
        }
    }
}
