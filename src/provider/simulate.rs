use alloy::{
    primitives::{Address, Bytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
use url::Url;

use crate::{
    utils::misc::{load_wallet_abi, stub_mode_enabled},
    Config,
};

pub async fn simulate_transaction(
    wallet_address: &str,
    calldata: &str,
    chain_id: u64,
    cfg: &Config,
) -> Result<u64, String> {
    if cfg.is_simulation_disabled() {
        return Ok(150_000);
    }

    if stub_mode_enabled() {
        return Ok(150_000);
    }

    let rpc_url = cfg
        .rpc_url_for_chain(&chain_id.to_string())
        .ok_or_else(|| format!("No RPC URL configured for chain {}", chain_id))?;

    let wallet_addr: Address = wallet_address
        .parse()
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    let calldata_bytes: Bytes = calldata
        .parse()
        .map_err(|e| format!("Invalid calldata format: {}", e))?;

    let abi = load_wallet_abi().map_err(|e| format!("Failed to load wallet ABI: {}", e))?;

    if calldata_bytes.len() < 4 {
        return Err("Calldata too short".to_string());
    }

    let function_selector = &calldata_bytes[..4];

    let execute_with_relayer_fn = abi
        .functions()
        .find(|f| f.name == "executeWithRelayer")
        .ok_or_else(|| "executeWithRelayer function not found in ABI".to_string())?;

    let expected_selector = execute_with_relayer_fn.selector();

    if function_selector != expected_selector.as_slice() {
        return Err(format!(
            "Transaction is not calling executeWithRelayer (expected: 0x{}, got: 0x{})",
            hex::encode(expected_selector),
            hex::encode(function_selector)
        ));
    }

    let rpc_endpoint = Url::parse(&rpc_url).map_err(|e| format!("Invalid RPC URL: {}", e))?;
    let provider = ProviderBuilder::new().on_hyper_http(rpc_endpoint);

    let tx = TransactionRequest::default()
        .to(wallet_addr)
        .input(calldata_bytes.into());

    if let Err(e) = provider.call(&tx).await {
        return Err(format!("Transaction simulation failed: {}", e));
    }

    match provider.estimate_gas(&tx).await {
        Ok(gas_estimate) => {
            tracing::info!(
                "Simulation OK for {} on chain {}, estimated gas: {}",
                wallet_address,
                chain_id,
                gas_estimate
            );
            Ok(gas_estimate)
        }
        Err(e) => Err(format!("Gas estimation failed: {}", e)),
    }
}
