use anyhow::Result;
use clap::Parser;
use relayx::{config::Config, rpc::RpcServer, storage::Storage};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let config = Config::parse();

    tracing::info!("Starting RelayX service with config: {:?}", config);

    // Initialize storage
    let storage = Storage::new(&config.db_path)?;

    // Create and start RPC server
    let rpc_host = config.rpc_host.clone();
    let rpc_port = config.rpc_port;
    let rpc_server = RpcServer::new(config.rpc_host, config.rpc_port, storage.clone())?;

    tracing::info!("RPC server started on {}:{}", rpc_host, rpc_port);

    // Start the RPC server
    rpc_server.start().await?;

    Ok(())
}
