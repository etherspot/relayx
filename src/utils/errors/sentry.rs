/// Capture an error in Sentry with context
pub fn capture_sentry_error(endpoint: &str, error: &jsonrpc_core::Error) {
    sentry::configure_scope(|scope| {
        scope.set_tag("endpoint", endpoint);
        scope.set_tag("error_code", format!("{:?}", error.code));
        scope.set_extra("error_message", error.message.clone().into());
    });
    sentry::capture_message(
        &format!("{} error: {}", endpoint, error.message),
        sentry::Level::Error,
    );
}
