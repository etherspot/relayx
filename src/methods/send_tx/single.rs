use crate::{
    methods::send_tx::shared::process_single_transaction, Config, SendTransactionParams, Storage,
};

pub async fn process_send_transaction(
    storage: Storage,
    params: &SendTransactionParams,
    cfg: &Config,
) -> Result<String, jsonrpc_core::Error> {
    tracing::info!("=== relayer_sendTransaction ===");
    process_single_transaction(&storage, params, cfg).await
}
