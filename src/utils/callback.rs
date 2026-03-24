use crate::{RelayerRequest, SpecStatusResponse};

/// POST the final status payload to the callback URL registered for a request.
///
/// The payload mirrors the `relayer_getStatus` response, with `taskId` added at the top level.
/// Failures are logged and silently swallowed — a failed callback never affects the relay flow.
pub async fn fire_callback(req: &RelayerRequest, status: &SpecStatusResponse) {
    let url = match &req.callback_url {
        Some(u) => u.clone(),
        None => return,
    };

    #[derive(serde::Serialize)]
    struct CallbackPayload<'a> {
        #[serde(rename = "taskId")]
        task_id: &'a str,
        #[serde(flatten)]
        status: &'a SpecStatusResponse,
    }

    let payload = CallbackPayload {
        task_id: &req.task_id,
        status,
    };

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
