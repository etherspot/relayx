use alloy::primitives::U256;

/// Convert a 0x-prefixed hex string to its decimal string representation.
/// Returns the input unchanged if it cannot be parsed, so callers always get a valid string.
pub fn hex_to_decimal(hex: &str) -> String {
    let s = hex.strip_prefix("0x").unwrap_or(hex);
    u128::from_str_radix(s, 16)
        .map(|v| v.to_string())
        .unwrap_or_else(|_| hex.to_string())
}

pub fn bump_gas_price_hex(gas_price_hex: &str, percent: u64) -> String {
    let s = gas_price_hex.strip_prefix("0x").unwrap_or(gas_price_hex);
    if let Ok(mut v) = u128::from_str_radix(s, 16) {
        v = v + (v * percent as u128 / 100u128);
        return format!("0x{:x}", v);
    }
    gas_price_hex.to_string()
}

pub fn parse_hex_u256(value: &str) -> Option<U256> {
    let trimmed = value.trim_start_matches("0x");
    if trimmed.is_empty() {
        return None;
    }
    U256::from_str_radix(trimmed, 16).ok()
}
