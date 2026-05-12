//! Guardrails for outbound status webhooks (`context.callbackUrl`).
//!
//! Mitigates SSRF (issue #30): restrict schemes, forbid URL credentials, block
//! non-public/reserved destinations by default, and resolve hostnames to ensure no
//! resolved address is disallowed.

use std::net::IpAddr;

use url::Url;

fn ssrf_checks_disabled() -> bool {
    matches!(
        std::env::var("RELAYX_CALLBACK_SKIP_SSRF_CHECKS")
            .map(|v| { matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on") })
            .unwrap_or(false),
        true
    )
}

fn allow_loopback_callback_targets() -> bool {
    matches!(
        std::env::var("RELAYX_CALLBACK_ALLOW_LOOPBACK")
            .map(|v| { matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on") })
            .unwrap_or(false),
        true
    )
}

/// True when this IP must not be used as a webhook target (strict default).
fn is_blocked_ip(ip: IpAddr) -> bool {
    if allow_loopback_callback_targets() && ip.is_loopback() {
        return false;
    }
    match ip {
        IpAddr::V4(v) => {
            v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_broadcast()
                || v.is_documentation()
                || v.is_unspecified()
        }
        IpAddr::V6(v) => {
            v.is_loopback()
                || v.is_unique_local()
                || v.is_unicast_link_local()
                || v.is_multicast()
                || v.is_unspecified()
        }
    }
}

/// Validate a client-supplied webhook URL before persisting the relay job.
///
/// Policy (unless `RELAYX_CALLBACK_SKIP_SSRF_CHECKS` is set):
/// - Only `https` URLs (no `http`, `file`, `gopher`, etc.).
/// - No username/password embedded in the URL.
/// - Literal IP hosts must not be loopback, private, link-local, documentation, etc.
/// - Domain hosts are resolved with [`tokio::net::lookup_host`]; every resolved address
///   must pass the same IP rules.
///
/// Set `RELAYX_CALLBACK_ALLOW_LOOPBACK=true` to permit loopback targets (local dev only).
pub async fn validate_outbound_webhook_url(raw: &str) -> Result<(), String> {
    if raw.len() > 2048 {
        return Err("callback URL exceeds maximum length".into());
    }

    if ssrf_checks_disabled() {
        Url::parse(raw).map_err(|e| format!("invalid URL: {e}"))?;
        return Ok(());
    }

    let url = Url::parse(raw).map_err(|e| format!("invalid URL: {e}"))?;

    if !url.username().is_empty() || url.password().is_some() {
        return Err("callback URL must not contain credentials".into());
    }

    if url.scheme() != "https" {
        return Err("only https callback URLs are allowed".into());
    }

    let host = url.host_str().ok_or("callback URL is missing a host")?;
    let port = url.port_or_known_default().unwrap_or(443);

    match url.host() {
        Some(url::Host::Ipv4(ip)) => {
            if is_blocked_ip(IpAddr::V4(ip)) {
                return Err("callback host IP is not an allowed public address".into());
            }
        }
        Some(url::Host::Ipv6(ip)) => {
            if is_blocked_ip(IpAddr::V6(ip)) {
                return Err("callback host IP is not an allowed public address".into());
            }
        }
        Some(url::Host::Domain(_)) => {
            let mut found = false;
            for sa in tokio::net::lookup_host((host, port))
                .await
                .map_err(|e| format!("DNS lookup failed for callback host: {e}"))?
            {
                found = true;
                if is_blocked_ip(sa.ip()) {
                    return Err(format!(
                        "callback host resolves to a disallowed address ({})",
                        sa.ip()
                    ));
                }
            }
            if !found {
                return Err("callback host resolved to no addresses".into());
            }
        }
        None => return Err("callback URL has an invalid host".into()),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_ipv4_detection() {
        assert!(is_blocked_ip(IpAddr::V4("127.0.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4("10.0.0.1".parse().unwrap())));
        assert!(is_blocked_ip(IpAddr::V4(
            "169.254.169.254".parse().unwrap()
        )));
        assert!(!is_blocked_ip(IpAddr::V4("8.8.8.8".parse().unwrap())));
    }

    #[tokio::test]
    async fn rejects_https_with_literal_private_ip() {
        let err = validate_outbound_webhook_url("https://10.0.0.1/webhook")
            .await
            .unwrap_err();
        assert!(err.contains("not an allowed public"));
    }

    #[tokio::test]
    async fn rejects_non_https_scheme() {
        let err = validate_outbound_webhook_url("http://8.8.8.8/webhook")
            .await
            .unwrap_err();
        assert!(err.contains("only https"));
    }

    #[tokio::test]
    async fn rejects_credentials_in_userinfo() {
        let err = validate_outbound_webhook_url("https://user:pass@8.8.8.8/hook")
            .await
            .unwrap_err();
        assert!(err.contains("credentials"));
    }
}
