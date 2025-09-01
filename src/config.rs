use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(name = "relayx")]
#[command(about = "A modular relayer service with JSON-RPC endpoints")]
pub struct Config {
    /// RPC server host address
    #[arg(long, default_value = "127.0.0.1")]
    pub rpc_host: String,

    /// RPC server port
    #[arg(long, default_value = "8545")]
    pub rpc_port: u16,

    /// Database path for RocksDB storage
    #[arg(long, default_value = "./relayx_db")]
    pub db_path: PathBuf,

    /// Comma-separated list of relayer addresses
    #[arg(long, default_value = "")]
    pub relayers: String,

    /// Maximum number of concurrent requests
    #[arg(long, default_value = "100")]
    pub max_concurrent_requests: usize,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    pub request_timeout: u64,
}

impl Config {
    /// Parse relayers string into a vector of addresses
    pub fn get_relayer_addresses(&self) -> Vec<String> {
        if self.relayers.is_empty() {
            return Vec::new();
        }

        self.relayers
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
