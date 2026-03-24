use chrono::Utc;

use crate::{Config, HealthResponse, RequestStatus, Storage};

pub async fn process_health_check(
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

    Ok(HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        uptime_seconds: storage.get_uptime_seconds(),
        total_requests,
        pending_requests,
        completed_requests,
        failed_requests,
    })
}
