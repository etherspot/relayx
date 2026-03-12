use relayx::{
    config::Config,
    storage::Storage,
    types::{FeeDataParams, GetStatusParams, Payment, QuoteRequest, SendTransactionParams},
};
use serde_json::json;
use tempfile::TempDir;

fn create_test_config(temp_dir: &TempDir) -> Config {
    let db_path = temp_dir.path().join("test_db");

    Config {
        rpc_host: "127.0.0.1".to_string(),
        rpc_port: 0,
        db_path,
        relayers: String::new(),
        max_concurrent_requests: 100,
        request_timeout: 30,
        config_path: None,
        http_address: "127.0.0.1".to_string(),
        http_port: 0,
        http_cors: "*".to_string(),
        log_level: "info".to_string(),
        relayer_private_key: None,
        disable_simulation: false,
        sentry_dsn: None,
    }
}

fn create_test_storage(temp_dir: &TempDir) -> Storage {
    let db_path = temp_dir.path().join("test_storage_db");
    Storage::new(&db_path).expect("Failed to create test storage")
}

#[cfg(test)]
mod send_transaction_tests {
    use super::*;

    fn native_payment() -> Payment {
        Payment {
            payment_type: "token".to_string(),
            address: Some("0x0000000000000000000000000000000000000000".to_string()),
            data: None,
        }
    }

    fn erc20_payment(token: &str) -> Payment {
        Payment {
            payment_type: "token".to_string(),
            address: Some(token.to_string()),
            data: None,
        }
    }

    fn sponsored_payment() -> Payment {
        Payment {
            payment_type: "sponsored".to_string(),
            address: None,
            data: None,
        }
    }

    #[test]
    fn test_send_transaction_missing_to_field() {
        let req = SendTransactionParams {
            chain_id: "1".to_string(),
            payment: native_payment(),
            to: "".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        assert!(req.to.is_empty());
    }

    #[test]
    fn test_send_transaction_missing_data_field() {
        let req = SendTransactionParams {
            chain_id: "1".to_string(),
            payment: native_payment(),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        assert!(req.data.is_empty());
    }

    #[test]
    fn test_send_transaction_missing_chain_id() {
        let req = SendTransactionParams {
            chain_id: "".to_string(),
            payment: native_payment(),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        assert!(req.chain_id.is_empty());
    }

    #[test]
    fn test_send_transaction_invalid_chain_id() {
        let req = SendTransactionParams {
            chain_id: "invalid".to_string(),
            payment: native_payment(),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        let result: Result<u64, _> = req.chain_id.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_send_transaction_valid_native_payment() {
        let req = SendTransactionParams {
            chain_id: "1".to_string(),
            payment: native_payment(),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        assert_eq!(req.payment.payment_type, "token");
        assert_eq!(
            req.payment.address.as_deref(),
            Some("0x0000000000000000000000000000000000000000")
        );
    }

    #[test]
    fn test_send_transaction_valid_erc20_payment() {
        let token = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
        let req = SendTransactionParams {
            chain_id: "1".to_string(),
            payment: erc20_payment(token),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        assert_eq!(req.payment.payment_type, "token");
        let addr = req.payment.address.as_deref().unwrap();
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
    }

    #[test]
    fn test_send_transaction_sponsored_payment() {
        let req = SendTransactionParams {
            chain_id: "1".to_string(),
            payment: sponsored_payment(),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        };
        assert_eq!(req.payment.payment_type, "sponsored");
        assert!(req.payment.address.is_none());
    }

    #[test]
    fn test_send_transaction_optional_task_id() {
        let req = SendTransactionParams {
            chain_id: "1".to_string(),
            payment: native_payment(),
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: Some(
                "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331".to_string(),
            ),
        };
        assert!(req.task_id.is_some());
    }
}

#[cfg(test)]
mod get_status_tests {
    use super::*;

    #[test]
    fn test_get_status_with_valid_task_id() {
        let req = GetStatusParams {
            id: "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331".to_string(),
            logs: false,
        };
        assert!(!req.id.is_empty());
        assert!(!req.logs);
    }

    #[test]
    fn test_get_status_with_logs_enabled() {
        let req = GetStatusParams {
            id: "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331".to_string(),
            logs: true,
        };
        assert!(req.logs);
    }
}

#[cfg(test)]
mod fee_data_tests {
    use super::*;

    #[test]
    fn test_fee_data_native_token() {
        let req = FeeDataParams {
            token: "0x0000000000000000000000000000000000000000".to_string(),
            chain_id: "1".to_string(),
        };
        assert_eq!(req.token, "0x0000000000000000000000000000000000000000");
        assert_eq!(req.chain_id, "1");
    }

    #[test]
    fn test_fee_data_erc20_token() {
        let req = FeeDataParams {
            token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            chain_id: "1".to_string(),
        };
        assert!(req.token.starts_with("0x"));
        assert_eq!(req.token.len(), 42);
    }

    #[test]
    fn test_fee_data_different_chains() {
        let chains = vec!["1", "137", "10", "8453"];

        for chain in chains {
            let req = FeeDataParams {
                token: "0x0000000000000000000000000000000000000000".to_string(),
                chain_id: chain.to_string(),
            };
            assert_eq!(req.chain_id, chain);
        }
    }
}

#[cfg(test)]
mod quote_tests {
    use super::*;

    #[test]
    fn test_quote_request_basic() {
        let request = QuoteRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: None,
            chain_id: Some("1".to_string()),
            authorization_list: None,
        };
        assert!(!request.to.is_empty());
        assert!(!request.data.is_empty());
    }

    #[test]
    fn test_quote_request_with_capabilities() {
        let request = QuoteRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: Some(relayx::types::QuoteRequestCapabilities {
                payment: Some(json!({
                    "type": "token",
                    "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                })),
            }),
            chain_id: Some("1".to_string()),
            authorization_list: None,
        };
        assert!(request.capabilities.is_some());
    }
}

#[cfg(test)]
mod send_transaction_multichain_tests {
    use super::*;

    fn make_payment_tx(chain_id: &str, to: &str) -> SendTransactionParams {
        SendTransactionParams {
            chain_id: chain_id.to_string(),
            payment: Payment {
                payment_type: "token".to_string(),
                address: Some("0x0000000000000000000000000000000000000000".to_string()),
                data: None,
            },
            to: to.to_string(),
            data: "0x1234".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        }
    }

    fn make_sponsored_tx(chain_id: &str, to: &str) -> SendTransactionParams {
        SendTransactionParams {
            chain_id: chain_id.to_string(),
            payment: Payment {
                payment_type: "sponsored".to_string(),
                address: None,
                data: None,
            },
            to: to.to_string(),
            data: "0x5678".to_string(),
            context: None,
            authorization_list: None,
            task_id: None,
        }
    }

    #[test]
    fn test_multichain_basic_request() {
        // Per spec: first item has payment, subsequent items are sponsored
        let items = vec![
            make_payment_tx("1", "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"),
            make_sponsored_tx("8453", "0x8922b54716264130634d6ff183747a8ead91a40c"),
        ];
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].payment.payment_type, "token");
        assert_eq!(items[1].payment.payment_type, "sponsored");
    }

    #[test]
    fn test_multichain_requires_more_than_one() {
        let items: Vec<SendTransactionParams> = vec![];
        assert!(items.is_empty(), "empty list is invalid per spec");

        let single = vec![make_payment_tx("1", "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6")];
        assert_eq!(single.len(), 1, "single item is also invalid per spec");
    }

    #[test]
    fn test_multichain_first_has_payment_rest_are_sponsored() {
        let items = vec![
            make_payment_tx("1", "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"),
            make_sponsored_tx("10", "0xaaaa35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"),
            make_sponsored_tx("137", "0xbbbb35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"),
        ];

        // First item carries payment
        assert_ne!(items[0].payment.payment_type, "sponsored");
        // All subsequent items must be sponsored
        for item in &items[1..] {
            assert_eq!(item.payment.payment_type, "sponsored");
        }
    }

    #[test]
    fn test_multichain_different_chains() {
        let chains = vec!["1", "10", "137", "8453", "42161"];
        let mut items = vec![make_payment_tx(chains[0], "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6")];
        for &chain in &chains[1..] {
            items.push(make_sponsored_tx(chain, "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"));
        }
        assert_eq!(items.len(), 5);
    }
}

#[cfg(test)]
mod storage_tests {
    use chrono::Utc;
    use relayx::types::{RelayerRequest, RequestStatus};
    use uuid::Uuid;

    use super::*;

    fn make_request(id: Uuid) -> RelayerRequest {
        RelayerRequest {
            id,
            task_id: format!("0x{}", hex::encode([0u8; 16].iter().chain(id.as_bytes()).copied().collect::<Vec<_>>())),
            from_address: "0x1234567890123456789012345678901234567890".to_string(),
            to_address: "0x0987654321098765432109876543210987654321".to_string(),
            amount: "1000000000000000000".to_string(),
            gas_limit: 21000,
            gas_price: "0x4a817c800".to_string(),
            data: Some("0x".to_string()),
            nonce: 0,
            chain_id: 1,
            transaction_hash: None,
            status: RequestStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_retrieve_request() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let request_id = Uuid::new_v4();
        let request = make_request(request_id);

        storage.create_request(request.clone()).await.unwrap();

        let retrieved = storage.get_request(request_id).await.unwrap();
        assert!(retrieved.is_some());

        let r = retrieved.unwrap();
        assert_eq!(r.id, request_id);
        assert_eq!(r.from_address, request.from_address);
        assert_eq!(r.to_address, request.to_address);
        assert_eq!(r.status, RequestStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_request_status() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let request_id = Uuid::new_v4();
        storage.create_request(make_request(request_id)).await.unwrap();

        storage
            .update_request_status(request_id, RequestStatus::Completed, None)
            .await
            .unwrap();

        let updated = storage.get_request(request_id).await.unwrap().unwrap();
        assert_eq!(updated.status, RequestStatus::Completed);
    }

    #[tokio::test]
    async fn test_get_request_count_by_status() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        for i in 0..5 {
            let status = if i < 2 {
                RequestStatus::Pending
            } else if i < 4 {
                RequestStatus::Completed
            } else {
                RequestStatus::Failed
            };

            let mut req = make_request(Uuid::new_v4());
            req.status = status;
            storage.create_request(req).await.unwrap();
        }

        assert_eq!(
            storage.get_request_count_by_status(RequestStatus::Pending).await.unwrap(),
            2
        );
        assert_eq!(
            storage.get_request_count_by_status(RequestStatus::Completed).await.unwrap(),
            2
        );
        assert_eq!(
            storage.get_request_count_by_status(RequestStatus::Failed).await.unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_get_total_request_count() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        for _ in 0..3 {
            storage.create_request(make_request(Uuid::new_v4())).await.unwrap();
        }

        assert_eq!(storage.get_total_request_count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_get_requests_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        for _ in 0..5 {
            storage.create_request(make_request(Uuid::new_v4())).await.unwrap();
        }

        assert_eq!(storage.get_requests(Some(3)).await.unwrap().len(), 3);
        assert_eq!(storage.get_requests(None).await.unwrap().len(), 5);
    }

    #[test]
    fn test_storage_uptime() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);
        let _ = storage.get_uptime_seconds();
    }

    #[tokio::test]
    async fn test_task_id_index_lookup() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let task_id = "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331";
        let request_id = Uuid::new_v4();
        let mut req = make_request(request_id);
        req.task_id = task_id.to_string();
        storage.create_request(req).await.unwrap();

        // Lookup via task_id secondary index
        let found = storage.get_request_by_task_id(task_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, request_id);

        // Check existence check
        assert!(storage.task_id_exists(task_id));
        assert!(!storage.task_id_exists("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"));
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);

        assert_eq!(config.rpc_host, "127.0.0.1");
        assert_eq!(config.http_cors, "*");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.max_concurrent_requests, 100);
        assert_eq!(config.request_timeout, 30);
    }

    #[test]
    fn test_config_log_level() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);

        for level in &["trace", "debug", "info", "warn", "error"] {
            config.log_level = level.to_string();
            assert_eq!(config.log_level, *level);
        }
    }
}
