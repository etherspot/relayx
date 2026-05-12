use std::sync::OnceLock;

use serde::Serialize;

use crate::{RelayerRequest, SpecStatusResponse};

fn webhook_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest webhook client builder")
    })
}

/// POST the final status payload to the callback URL registered for a request.
///
/// The payload mirrors the `relayer_getStatus` response, with `taskId` added at the top level.
/// The HTTP client does not follow redirects (mitigates SSRF redirect chains; issue #30).
///
/// Failures are logged and silently swallowed — a failed callback never affects the relay flow.
pub async fn fire_callback(req: &RelayerRequest, status: &SpecStatusResponse) {
    let url = match &req.callback_url {
        Some(u) if !u.is_empty() => u.clone(),
        _ => return,
    };

    #[derive(Serialize)]
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

    match webhook_http_client().post(&url).json(&payload).send().await {
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
