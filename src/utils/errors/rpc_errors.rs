pub fn invalid_params_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::InvalidParams);
    err.message = "Invalid params".to_string();
    err
}

pub fn unsupported_payment_token_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4202));
    err.message = "Unsupported Payment Token".to_string();
    err
}

pub fn insufficient_balance_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4205));
    err.message = "Insufficient Balance".to_string();
    err
}

pub fn unsupported_chain_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4206));
    err.message = "Unsupported Chain".to_string();
    err
}

pub fn unknown_transaction_id_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4208));
    err.message = "Unknown Transaction ID".to_string();
    err
}

pub fn unsupported_capability_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4209));
    err.message = "Unsupported Capability".to_string();
    err
}

pub fn invalid_authorization_list_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4210));
    err.message = "Invalid Authorization List".to_string();
    err
}

pub fn simulation_failed_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4211));
    err.message = "Simulation Failed".to_string();
    err
}

pub fn invalid_task_id_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4213));
    err.message = "Invalid Task ID".to_string();
    err
}

pub fn duplicate_task_id_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4214));
    err.message = "Duplicate Task ID".to_string();
    err
}

pub fn quote_expired_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4204));
    err.message = "Quote Expired".to_string();
    err
}

pub fn transaction_too_large_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4207));
    err.message = "Transaction Too Large".to_string();
    err
}

pub fn multichain_not_supported_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4212));
    err.message = "Multichain Not Supported".to_string();
    err
}

// ===== Spec-compliant error helpers (positive codes per spec) =====

/// 4200: Raised by fee-verification middleware when the on-chain transfer amount is below the
/// minimum required fee (spec §Error Codes). Not wired to the core relay path — callers that
/// inspect ERC-20 transfer events before relaying should return this.
pub fn insufficient_payment_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4200));
    err.message = "Insufficient Payment".to_string();
    err
}

/// 4201: Raised when signature recovery fails or the recovered signer does not match the account.
/// Returned by validator middleware that performs off-chain signature verification before relaying.
pub fn invalid_signature_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4201));
    err.message = "Invalid Signature".to_string();
    err
}

/// 4203: Raised by rate-limiting middleware when a caller exceeds the per-address or per-API-key
/// request quota. Operators integrating a rate-limiter layer should return this error.
pub fn rate_limit_exceeded_error() -> jsonrpc_core::Error {
    let mut err = jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(4203));
    err.message = "Rate Limit Exceeded".to_string();
    err
}
