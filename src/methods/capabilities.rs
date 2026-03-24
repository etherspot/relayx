use std::collections::HashMap;

use crate::{ChainCapabilities, Config, GetCapabilitiesResponse, Storage};

pub async fn process_get_capabilities(
    _storage: Storage,
    params_chains: &[String],
    cfg: &Config,
) -> Result<GetCapabilitiesResponse, jsonrpc_core::Error> {
    tracing::info!("=== relayer_getCapabilities chains={:?} ===", params_chains);

    let fee_collector = std::env::var("RELAYX_FEE_COLLECTOR")
        .ok()
        .or_else(|| cfg.fee_collector())
        .unwrap_or_else(|| "0x55f3a93f544e01ce4378d25e927d7c493b863bd6".to_string());

    // let supported_tokens = cfg.get_supported_tokens();
    let supported_chains = cfg.supported_chain_ids();

    // If specific chain IDs requested, return only those; otherwise use all configured chains
    let mut result: HashMap<String, ChainCapabilities> = HashMap::new();

    let chains_to_process: Vec<String> = if params_chains.is_empty() {
        if supported_chains.is_empty() {
            vec!["1".to_string()]
        } else {
            supported_chains.clone()
        }
    } else {
        params_chains.to_vec()
    };

    chains_to_process.iter().for_each(|chain_id| {
        let supported_tokens = cfg.get_supported_token(chain_id);
        result.insert(
            chain_id.to_string(),
            ChainCapabilities {
                fee_collector: fee_collector.clone(),
                tokens: supported_tokens,
            },
        );
    });

    Ok(result)
}
