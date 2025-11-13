use std::{fs, path::PathBuf, sync::OnceLock};

use clap::Parser;
use serde::{Deserialize, Serialize};

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

    /// Path to external JSON config file containing RPC URLs per chainId
    /// Can also be provided via RELAYX_CONFIG environment variable
    #[arg(long = "config", env = "RELAYX_CONFIG")]
    pub config_path: Option<PathBuf>,

    /// HTTP bind address (overridable by config.json)
    #[arg(
        long = "http-address",
        env = "HTTP_ADDRESS",
        default_value = "127.0.0.1"
    )]
    pub http_address: String,

    /// HTTP bind port (overridable by config.json)
    #[arg(long = "http-port", env = "HTTP_PORT", default_value_t = 4937)]
    pub http_port: u16,

    /// HTTP CORS setting: "*" or comma-separated origins (overridable by config.json)
    #[arg(long = "http-cors", env = "HTTP_CORS", default_value = "*")]
    pub http_cors: String,

    /// Log level: trace, debug, info, warn, error
    #[arg(long = "log-level", env = "LOG_LEVEL", default_value = "debug")]
    pub log_level: String,

    /// Relayer private key used for signing transactions
    #[arg(long = "relayer-private-key", env = "RELAYX_PRIVATE_KEY")]
    pub relayer_private_key: Option<String>,
}

impl Config {
    /// Cached parsed JSON config (loaded once globally)
    fn get_json_config(&self) -> Option<&'static serde_json::Value> {
        static JSON_CONFIG: OnceLock<serde_json::Value> = OnceLock::new();
        let value = JSON_CONFIG.get_or_init(|| {
            let path = match &self.config_path {
                Some(p) => p.clone(),
                None => match std::env::var("RELAYX_CONFIG").ok() {
                    Some(s) => PathBuf::from(s),
                    None => return serde_json::Value::Null,
                },
            };
            match fs::read_to_string(path) {
                Ok(content) => serde_json::from_str::<serde_json::Value>(&content)
                    .unwrap_or(serde_json::Value::Null),
                Err(_) => serde_json::Value::Null,
            }
        });
        if value.is_null() {
            None
        } else {
            Some(value)
        }
    }
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

    /// Look up the RPC URL for a given chainId by reading the configured JSON file.
    /// The file path is taken from `--config` CLI flag or the `RELAYX_CONFIG` env var.
    /// Supported JSON formats:
    /// 1) { "1": "https://mainnet.example", "137": "https://polygon.example" }
    /// 2) { "rpcs": { "1": "https://mainnet.example" } }
    pub fn rpc_url_for_chain(&self, chain_id: &str) -> Option<String> {
        let root = self.get_json_config()?;
        // Try { "rpcs": { chainId: url } }
        if let Some(url) = root
            .get("rpcs")
            .and_then(|m| m.get(chain_id))
            .and_then(|v| v.as_str())
        {
            return Some(url.to_string());
        }
        // Try flat map { chainId: url }
        if let Some(url) = root.get(chain_id).and_then(|v| v.as_str()) {
            return Some(url.to_string());
        }
        None
    }

    /// Returns the configured fee collector address if present in the JSON file.
    /// Supports either top-level `feeCollector` or nested `{ "feeCollector": "0x..." }` alongside
    /// `rpcs`.
    pub fn fee_collector(&self) -> Option<String> {
        let root = self.get_json_config()?;
        root.get("feeCollector")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Returns Chainlink native token/USD aggregator address for a chain
    /// Expects JSON structure: { "chainlink": { "nativeUsd": { "1": "0x..." } } }
    pub fn chainlink_native_usd(&self, chain_id: &str) -> Option<String> {
        let root = self.get_json_config()?;
        root.get("chainlink")
            .and_then(|c| c.get("nativeUsd"))
            .and_then(|m| m.get(chain_id))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Returns Chainlink token/USD aggregator address for a given chain and token address
    /// Expects JSON structure: { "chainlink": { "tokenUsd": { "1": { "0xToken": "0xFeed" } } } }
    pub fn chainlink_token_usd(&self, chain_id: &str, token_address: &str) -> Option<String> {
        let root = self.get_json_config()?;
        let token_lc = token_address.to_lowercase();
        root.get("chainlink")
            .and_then(|c| c.get("tokenUsd"))
            .and_then(|chains| chains.get(chain_id))
            .and_then(|map| map.get(&token_lc))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Effective HTTP address from config.json or CLI
    pub fn get_http_address(&self) -> String {
        self.get_json_config()
            .and_then(|v| {
                v.get("http_address")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| self.http_address.clone())
    }

    /// Effective HTTP port from config.json or CLI
    pub fn get_http_port(&self) -> u16 {
        self.get_json_config()
            .and_then(|v| {
                v.get("http_port")
                    .and_then(|n| n.as_u64())
                    .and_then(|n| u16::try_from(n).ok())
            })
            .unwrap_or(self.http_port)
    }

    /// Effective CORS setting from config.json or CLI
    pub fn get_http_cors(&self) -> String {
        self.get_json_config()
            .and_then(|v| {
                v.get("http_cors")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| self.http_cors.clone())
    }

    /// Returns the configured default token address if present in the JSON file or environment.
    /// Supports either top-level `defaultToken` in config.json or `RELAYX_DEFAULT_TOKEN` env var.
    pub fn default_token(&self) -> Option<String> {
        // First check environment variable
        if let Ok(token) = std::env::var("RELAYX_DEFAULT_TOKEN") {
            if !token.is_empty() {
                return Some(token);
            }
        }

        // Then check config file
        let root = self.get_json_config()?;
        root.get("defaultToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Returns the relayer private key from CLI/env/config, if provided.
    pub fn get_relayer_private_key(&self) -> Option<String> {
        if let Some(cli_key) = self
            .relayer_private_key
            .as_ref()
            .filter(|s| !s.is_empty())
            .cloned()
        {
            return Some(cli_key);
        }

        if let Ok(env_key) = std::env::var("RELAYX_PRIVATE_KEY") {
            if !env_key.is_empty() {
                return Some(env_key);
            }
        }

        let root = self.get_json_config()?;
        root.get("relayerPrivateKey")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Returns all supported ERC20 token addresses from the chainlink configuration.
    /// This extracts tokens from the chainlink.tokenUsd configuration across all chains.
    pub fn get_supported_tokens(&self) -> Vec<String> {
        let root = match self.get_json_config() {
            Some(config) => config,
            None => return Vec::new(),
        };

        let mut tokens = Vec::new();

        // Extract tokens from chainlink.tokenUsd configuration
        if let Some(chainlink) = root.get("chainlink") {
            if let Some(token_usd) = chainlink.get("tokenUsd") {
                if let Some(token_map) = token_usd.as_object() {
                    for (_chain_id, chain_tokens) in token_map {
                        if let Some(chain_tokens_obj) = chain_tokens.as_object() {
                            for (token_address, _feed_address) in chain_tokens_obj {
                                tokens.push(token_address.clone());
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and sort for consistency
        tokens.sort();
        tokens.dedup();
        tokens
    }

    /// Check if a chain ID is supported by checking if it has an RPC URL configured
    pub fn is_chain_supported(&self, chain_id: u64) -> bool {
        self.rpc_url_for_chain(&chain_id.to_string()).is_some()
    }

    /// Get the effective log level from config.json or CLI
    pub fn get_log_level(&self) -> String {
        self.get_json_config()
            .and_then(|v| {
                v.get("log_level")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| self.log_level.clone())
    }

    /// Returns the configured Etherscan API key if present in the JSON file.
    /// Supports either top-level `etherscanApiKey` in config.json or `ETHERSCAN_API_KEY` env var.
    pub fn etherscan_api_key(&self) -> Option<String> {
        // First check environment variable
        if let Ok(key) = std::env::var("ETHERSCAN_API_KEY") {
            if !key.is_empty() {
                return Some(key);
            }
        }

        // Then check config file
        let root = self.get_json_config()?;
        root.get("etherscanApiKey")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Returns the configured Etherscan API base URL if present.
    /// Supports `ETHERSCAN_API_BASE` env var or `etherscanApiBase` in config.json.
    /// Defaults to "https://api.etherscan.io/v2/api".
    pub fn etherscan_api_base(&self) -> String {
        if let Ok(base) = std::env::var("ETHERSCAN_API_BASE") {
            if !base.is_empty() {
                return base;
            }
        }
        self.get_json_config()
            .and_then(|v| {
                v.get("etherscanApiBase")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "https://api.etherscan.io/v2/api".to_string())
    }
}
