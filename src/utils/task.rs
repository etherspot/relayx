use uuid::Uuid;

use crate::{
    utils::errors::rpc_errors::{duplicate_task_id_error, invalid_task_id_error},
    Storage,
};

/// Generate a random 32-byte task ID as a 0x-prefixed hex string.
/// Uses two UUID v4 values concatenated to produce 32 bytes.
pub fn generate_task_id() -> String {
    let b1 = Uuid::new_v4();
    let b2 = Uuid::new_v4();
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(b1.as_bytes());
    bytes[16..].copy_from_slice(b2.as_bytes());
    format!("0x{}", hex::encode(bytes))
}

/// Validate the format of a client-provided task ID.
/// Must be a 0x-prefixed 64-character hex string (32 bytes).
pub fn is_valid_task_id(id: &str) -> bool {
    if let Some(hex_part) = id.strip_prefix("0x") {
        hex_part.len() == 64 && hex_part.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        false
    }
}

/// Resolve (or generate) the task ID for a request, applying spec validation rules.
pub fn resolve_task_id(
    provided: Option<&str>,
    storage: &Storage,
) -> Result<String, jsonrpc_core::Error> {
    match provided {
        None => Ok(generate_task_id()),
        Some(id) => {
            if !is_valid_task_id(id) {
                return Err(invalid_task_id_error());
            }
            if storage.task_id_exists(id) {
                return Err(duplicate_task_id_error());
            }
            Ok(id.to_string())
        }
    }
}
