use uuid::Uuid;

use crate::{
    provider::fetch::fetch_receipt_for_status,
    utils::errors::rpc_errors::unknown_transaction_id_error, Config, GetStatusParams,
    RequestStatus, SpecStatusResponse, Storage,
};

pub async fn process_get_status(
    storage: Storage,
    params: &GetStatusParams,
    cfg: &Config,
) -> Result<SpecStatusResponse, jsonrpc_core::Error> {
    tracing::info!("=== relayer_getStatus id={} ===", params.id);

    // Look up by task_id first, then fall back to UUID for backward compatibility
    let req_opt = storage
        .get_request_by_task_id(&params.id)
        .await
        .map_err(|_| jsonrpc_core::Error::internal_error())?;

    let req_opt = if req_opt.is_none() {
        // Backward compat: try parsing as UUID
        if let Ok(uuid) = Uuid::parse_str(&params.id) {
            storage
                .get_request(uuid)
                .await
                .map_err(|_| jsonrpc_core::Error::internal_error())?
        } else {
            None
        }
    } else {
        req_opt
    };

    let req = match req_opt {
        Some(r) => r,
        None => return Err(unknown_transaction_id_error()),
    };

    let chain_id_str = req.chain_id.to_string();
    let created_at = req.created_at.timestamp() as u64;

    let response = match req.status {
        RequestStatus::Pending => SpecStatusResponse {
            chain_id: chain_id_str,
            created_at,
            status: 100,
            hash: None,
            receipt: None,
            message: None,
            data: None,
        },
        RequestStatus::Processing => {
            let hash = req.transaction_hash.clone();
            SpecStatusResponse {
                chain_id: chain_id_str,
                created_at,
                status: 110,
                hash,
                receipt: None,
                message: None,
                data: None,
            }
        }
        RequestStatus::Completed => {
            // Retrieve stored receipt if available; try fetching on-demand if not
            let mut receipt = storage.get_receipt(req.id).await.ok().flatten();

            if receipt.is_none() {
                if let Some(tx_hash) = &req.transaction_hash {
                    receipt = fetch_receipt_for_status(tx_hash, req.chain_id, cfg).await;
                }
            }

            // Strip logs if not requested
            if let Some(ref mut r) = receipt {
                if !params.logs {
                    r.logs = None;
                }
            }

            SpecStatusResponse {
                chain_id: chain_id_str,
                created_at,
                status: 200,
                hash: None,
                receipt,
                message: None,
                data: None,
            }
        }
        RequestStatus::Failed => {
            let msg = req.error_message.clone();
            SpecStatusResponse {
                chain_id: chain_id_str,
                created_at,
                status: 500,
                hash: req.transaction_hash.clone(),
                receipt: None,
                message: msg,
                data: None,
            }
        }
    };

    Ok(response)
}
