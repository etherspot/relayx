use anyhow::Result;
use clap::Parser;
use relayx::{config::Config, rpc::RpcServer, storage::Storage};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments first to get log level
    let config = Config::parse();

    // Get the configured log level
    let log_level = config.get_log_level();

    // Parse the log level string
    let filter_str = match log_level.to_lowercase().as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => {
            eprintln!("Invalid log level '{}', defaulting to 'debug'", log_level);
            "debug"
        }
    };

    // Initialize logging - respect RUST_LOG env var first, then use config
    // Format: "relayx=level" to include all logs from the relayx crate
    let env_filter = if std::env::var("RUST_LOG").is_ok() {
        // RUST_LOG is set, use it
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        // Use config value, format to include all modules from relayx crate and dependencies
        EnvFilter::new(&format!("relayx={},{}", filter_str, filter_str))
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // Initialize Sentry if DSN is provided (after tracing is set up)
    // Note: With the "panic" feature enabled, panics are automatically captured
    let _sentry_guard = if let Some(dsn) = config.get_sentry_dsn() {
        tracing::info!("Initializing Sentry error tracking");
        let guard = sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                ..Default::default()
            },
        ));
        
        tracing::info!("✓ Sentry initialized successfully (panics will be automatically captured)");
        Some(guard)
    } else {
        tracing::debug!("Sentry DSN not provided, skipping error tracking initialization");
        None
    };

    tracing::info!("Starting RelayX service");
    tracing::debug!("Configuration: {:?}", config);
    tracing::info!("Log level set to: {}", filter_str);

    // Initialize storage
    tracing::info!("Initializing storage at: {:?}", config.db_path);
    let storage = Storage::new(&config.db_path)?;
    tracing::info!("Storage initialized successfully");

    // Create and start RPC server
    let rpc_host = config.get_http_address();
    let rpc_port = config.get_http_port();

    tracing::info!("Creating RPC server on {}:{}", rpc_host, rpc_port);
    tracing::debug!("CORS configuration: {}", config.get_http_cors());

    let rpc_server = RpcServer::new(rpc_host.clone(), rpc_port, storage.clone(), config.clone())?;

    tracing::info!("✓ RPC server initialized successfully");
    tracing::info!("✓ Server listening on {}:{}", rpc_host, rpc_port);
    tracing::info!("✓ RelayX service ready to accept requests");

    // Start the RPC server
    rpc_server.start().await?;

    Ok(())
}
