use crate::{
    methods::send_tx::shared::process_single_transaction,
    utils::{
        errors::rpc_errors::{
            duplicate_task_id_error, invalid_params_error, invalid_task_id_error,
            multichain_not_supported_error,
        },
        task::is_valid_task_id,
    },
    Config, SendTransactionParams, Storage,
};

pub async fn process_send_transaction_multichain(
    storage: Storage,
    items: &[SendTransactionParams],
    cfg: &Config,
) -> Result<Vec<String>, jsonrpc_core::Error> {
    tracing::info!(
        "=== relayer_sendTransactionMultichain ({} items) ===",
        items.len()
    );

    // 4212: operator has disabled multichain support for this instance.
    if cfg.is_multichain_disabled() {
        return Err(multichain_not_supported_error());
    }

    if items.len() < 2 {
        return Err(jsonrpc_core::Error::invalid_params(
            "relayer_sendTransactionMultichain requires at least 2 transactions",
        ));
    }

    // First transaction must carry the payment (non-sponsored)
    if items[0].payment.payment_type == "sponsored" {
        return Err(invalid_params_error());
    }

    // All subsequent transactions must be sponsored
    for (idx, item) in items[1..].iter().enumerate() {
        if item.payment.payment_type != "sponsored" {
            return Err(jsonrpc_core::Error::invalid_params(format!(
                "Transaction {} (index {}) must use sponsored payment type",
                idx + 2,
                idx + 1
            )));
        }
    }

    // Validate all task IDs upfront before processing any transaction
    for item in items {
        if let Some(tid) = &item.task_id {
            if !is_valid_task_id(tid) {
                return Err(invalid_task_id_error());
            }
            if storage.task_id_exists(tid) {
                return Err(duplicate_task_id_error());
            }
        }
    }

    let mut task_ids = Vec::new();
    for item in items {
        let task_id = process_single_transaction(&storage, item, cfg).await?;
        task_ids.push(task_id);
    }

    Ok(task_ids)
}
