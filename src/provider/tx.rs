use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use url::Url;
use uuid::Uuid;

use crate::{
    utils::misc::{get_relayer_private_key, stub_mode_enabled},
    Config,
};

pub async fn send_relay_transaction(
    wallet_address: &str,
    calldata: &str,
    chain_id: u64,
    gas_limit: u64,
    gas_price_hex: &str,
    cfg: &Config,
) -> Result<String, String> {
    tracing::info!(
        "Preparing relay transaction to wallet {} on chain {}",
        wallet_address,
        chain_id
    );

    if stub_mode_enabled() {
        let fake_hash = format!("0x{}", hex::encode(Uuid::new_v4().as_bytes()));
        return Ok(fake_hash);
    }

    let private_key = get_relayer_private_key(cfg)?;
    let signer = private_key
        .parse::<PrivateKeySigner>()
        .map_err(|e| format!("Failed to parse private key: {}", e))?;
    let relayer_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let rpc_url = cfg
        .rpc_url_for_chain(&chain_id.to_string())
        .ok_or_else(|| format!("No RPC URL configured for chain {}", chain_id))?;

    let rpc_endpoint = Url::parse(&rpc_url).map_err(|e| format!("Invalid RPC URL: {}", e))?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_hyper_http(rpc_endpoint);

    let to_address: Address = wallet_address
        .parse()
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    let calldata_bytes = if let Some(stripped) = calldata.strip_prefix("0x") {
        hex::decode(stripped).map_err(|e| format!("Invalid calldata hex: {}", e))?
    } else {
        hex::decode(calldata).map_err(|e| format!("Invalid calldata hex: {}", e))?
    };

    let gas_price_value = if let Some(stripped) = gas_price_hex.strip_prefix("0x") {
        u128::from_str_radix(stripped, 16).map_err(|e| format!("Invalid gas price hex: {}", e))?
    } else {
        u128::from_str_radix(gas_price_hex, 16)
            .map_err(|e| format!("Invalid gas price hex: {}", e))?
    };

    let nonce = provider
        .get_transaction_count(relayer_address)
        .await
        .map_err(|e| format!("Failed to get nonce: {}", e))?;

    let mut tx = TransactionRequest::default()
        .to(to_address)
        .input(calldata_bytes.into())
        .gas_limit(gas_limit);

    tx.nonce = Some(nonce);
    tx.gas_price = Some(gas_price_value);
    tx.chain_id = Some(chain_id);

    match provider.send_transaction(tx).await {
        Ok(pending_tx) => {
            let tx_hash_hex = format!("0x{:x}", pending_tx.tx_hash());
            tracing::info!(
                "Transaction sent - Hash: {}, Chain: {}",
                tx_hash_hex,
                chain_id
            );
            Ok(tx_hash_hex)
        }
        Err(e) => {
            let error_msg = format!("Failed to send transaction: {}", e);
            tracing::error!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Error);
            Err(error_msg)
        }
    }
}
