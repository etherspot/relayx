use std::sync::OnceLock;

use alloy::{json_abi::JsonAbi, primitives::Address};

use crate::{
    utils::errors::rpc_errors::invalid_authorization_list_error, AuthorizationItem, Config,
};

pub fn stub_mode_enabled() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| {
        std::env::var("RELAYX_STUB_MODE")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    })
}

/// Validate a JSON authorization list against the expected chain_id and contract address.
pub fn validate_authorization_list(
    auth_list: &[AuthorizationItem],
    chain_id: u64,
    contract_address: Address,
) -> Result<(), jsonrpc_core::Error> {
    for auth in auth_list {
        if auth.chain_id != 0 && auth.chain_id != chain_id {
            tracing::warn!(
                "Authorization chain mismatch: expected {} or 0, found {}",
                chain_id,
                auth.chain_id
            );
            return Err(invalid_authorization_list_error());
        }

        let addr: Address = auth.address.parse().map_err(|_| {
            tracing::warn!("Invalid address in authorization list: {}", auth.address);
            invalid_authorization_list_error()
        })?;

        if addr != contract_address {
            tracing::warn!(
                "Authorization target mismatch: expected {}, found {}",
                contract_address,
                auth.address
            );
            return Err(invalid_authorization_list_error());
        }
    }
    Ok(())
}

pub fn load_wallet_abi() -> Result<JsonAbi, anyhow::Error> {
    let abi_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("abi.json");

    let abi_content = std::fs::read_to_string(&abi_path)
        .map_err(|e| anyhow::anyhow!("Failed to read ABI file at {:?}: {}", abi_path, e))?;

    let abi_json: serde_json::Value = serde_json::from_str(&abi_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI JSON: {}", e))?;

    let abi_array = abi_json
        .get("abi")
        .ok_or_else(|| anyhow::anyhow!("ABI JSON missing 'abi' field"))?;

    let abi: JsonAbi = serde_json::from_value(abi_array.clone())
        .map_err(|e| anyhow::anyhow!("Failed to deserialize ABI: {}", e))?;

    Ok(abi)
}

pub fn get_relayer_private_key(cfg: &Config) -> Result<String, String> {
    cfg.get_relayer_private_key()
        .ok_or_else(|| "RELAYX_PRIVATE_KEY configuration missing".to_string())
}
