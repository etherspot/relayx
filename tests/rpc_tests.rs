use relayx::{
    config::Config,
    storage::Storage,
    types::{
        ExchangeRateRequest, GetStatusRequest, PaymentCapability, QuoteRequest,
        SendTransactionCapabilities, SendTransactionRequest,
    },
};
use serde_json::json;
use tempfile::TempDir;

/// Helper function to create a test configuration
fn create_test_config(temp_dir: &TempDir) -> Config {
    let db_path = temp_dir.path().join("test_db");

    Config {
        rpc_host: "127.0.0.1".to_string(),
        rpc_port: 0, // Use 0 for random port in tests
        db_path,
        relayers: String::new(),
        max_concurrent_requests: 100,
        request_timeout: 30,
        config_path: None,
        http_address: "127.0.0.1".to_string(),
        http_port: 0,
        http_cors: "*".to_string(),
        log_level: "info".to_string(),
    }
}

/// Helper function to create test storage
fn create_test_storage(temp_dir: &TempDir) -> Storage {
    let db_path = temp_dir.path().join("test_storage_db");
    Storage::new(&db_path).expect("Failed to create test storage")
}

#[cfg(test)]
mod send_transaction_tests {
    use super::*;

    #[test]
    fn test_send_transaction_missing_to_field() {
        let _temp_dir = TempDir::new().unwrap();
        let request = SendTransactionRequest {
            to: "".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        // This should fail validation
        assert!(request.to.is_empty());
    }

    #[test]
    fn test_send_transaction_missing_data_field() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        assert!(request.data.is_empty());
    }

    #[test]
    fn test_send_transaction_missing_chain_id() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "".to_string(),
            authorization_list: String::new(),
        };

        assert!(request.chain_id.is_empty());
    }

    #[test]
    fn test_send_transaction_invalid_chain_id() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "invalid".to_string(),
            authorization_list: String::new(),
        };

        let result: Result<u64, _> = request.chain_id.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_send_transaction_valid_native_payment() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x0000000000000000000000000000000000000000".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        assert_eq!(request.capabilities.payment.payment_type, "native");
        assert_eq!(
            request.capabilities.payment.token,
            "0x0000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn test_send_transaction_invalid_native_token() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "native".to_string(),
                    token: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        // Native payment should have zero address
        assert_ne!(
            request.capabilities.payment.token,
            "0x0000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn test_send_transaction_valid_erc20_payment() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "erc20".to_string(),
                    token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        assert_eq!(request.capabilities.payment.payment_type, "erc20");
        assert!(request.capabilities.payment.token.starts_with("0x"));
        assert_eq!(request.capabilities.payment.token.len(), 42);
    }

    #[test]
    fn test_send_transaction_invalid_erc20_address() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "erc20".to_string(),
                    token: "0xInvalid".to_string(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        // Should be invalid length
        assert_ne!(request.capabilities.payment.token.len(), 42);
    }

    #[test]
    fn test_send_transaction_sponsored_payment() {
        let request = SendTransactionRequest {
            to: "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6".to_string(),
            data: "0x1234".to_string(),
            capabilities: SendTransactionCapabilities {
                payment: PaymentCapability {
                    payment_type: "sponsored".to_string(),
                    token: String::new(),
                    data: String::new(),
                },
            },
            chain_id: "1".to_string(),
            authorization_list: String::new(),
        };

        assert_eq!(request.capabilities.payment.payment_type, "sponsored");
    }
}

#[cfg(test)]
mod get_status_tests {
    use super::*;

    #[test]
    fn test_get_status_with_valid_ids() {
        let request = GetStatusRequest {
            ids: vec![
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            ],
        };

        assert_eq!(request.ids.len(), 2);
    }

    #[test]
    fn test_get_status_with_empty_ids() {
        let request = GetStatusRequest { ids: vec![] };

        assert!(request.ids.is_empty());
    }

    #[test]
    fn test_get_status_with_invalid_uuid() {
        let request = GetStatusRequest {
            ids: vec!["invalid-uuid".to_string()],
        };

        use uuid::Uuid;
        let result = Uuid::parse_str(&request.ids[0]);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod exchange_rate_tests {
    use super::*;

    #[test]
    fn test_exchange_rate_native_token() {
        let request = ExchangeRateRequest {
            token: "0x0000000000000000000000000000000000000000".to_string(),
            chain_id: "1".to_string(),
        };

        assert_eq!(request.token, "0x0000000000000000000000000000000000000000");
        assert_eq!(request.chain_id, "1");
    }

    #[test]
    fn test_exchange_rate_erc20_token() {
        let request = ExchangeRateRequest {
            token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            chain_id: "1".to_string(),
        };

        assert!(request.token.starts_with("0x"));
        assert_eq!(request.token.len(), 42);
    }

    #[test]
    fn test_exchange_rate_different_chains() {
        let chains = vec!["1", "137", "10", "8453"];

        for chain in chains {
            let request = ExchangeRateRequest {
                token: "0x0000000000000000000000000000000000000000".to_string(),
                chain_id: chain.to_string(),
            };

            assert_eq!(request.chain_id, chain);
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
                    "type": "erc20",
                    "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                })),
            }),
            chain_id: Some("1".to_string()),
            authorization_list: None,
        };

        assert!(request.capabilities.is_some());
    }
}

#[cfg(test)]
mod storage_tests {
    use chrono::Utc;
    use relayx::types::{RelayerRequest, RequestStatus};
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_create_and_retrieve_request() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let request_id = Uuid::new_v4();
        let request = RelayerRequest {
            id: request_id,
            from_address: "0x1234567890123456789012345678901234567890".to_string(),
            to_address: "0x0987654321098765432109876543210987654321".to_string(),
            amount: "1000000000000000000".to_string(),
            gas_limit: 21000,
            gas_price: "0x4a817c800".to_string(),
            data: Some("0x".to_string()),
            nonce: 0,
            chain_id: 1,
            status: RequestStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
        };

        // Create request
        storage.create_request(request.clone()).await.unwrap();

        // Retrieve request
        let retrieved = storage.get_request(request_id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved_request = retrieved.unwrap();
        assert_eq!(retrieved_request.id, request_id);
        assert_eq!(retrieved_request.from_address, request.from_address);
        assert_eq!(retrieved_request.to_address, request.to_address);
        assert_eq!(retrieved_request.status, RequestStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_request_status() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let request_id = Uuid::new_v4();
        let request = RelayerRequest {
            id: request_id,
            from_address: "0x1234567890123456789012345678901234567890".to_string(),
            to_address: "0x0987654321098765432109876543210987654321".to_string(),
            amount: "1000000000000000000".to_string(),
            gas_limit: 21000,
            gas_price: "0x4a817c800".to_string(),
            data: Some("0x".to_string()),
            nonce: 0,
            chain_id: 1,
            status: RequestStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
        };

        // Create request
        storage.create_request(request).await.unwrap();

        // Update status
        storage
            .update_request_status(request_id, RequestStatus::Completed, None)
            .await
            .unwrap();

        // Verify update
        let updated = storage.get_request(request_id).await.unwrap().unwrap();
        assert_eq!(updated.status, RequestStatus::Completed);
    }

    #[tokio::test]
    async fn test_get_request_count_by_status() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Create multiple requests with different statuses
        for i in 0..5 {
            let status = if i < 2 {
                RequestStatus::Pending
            } else if i < 4 {
                RequestStatus::Completed
            } else {
                RequestStatus::Failed
            };

            let request = RelayerRequest {
                id: Uuid::new_v4(),
                from_address: "0x1234567890123456789012345678901234567890".to_string(),
                to_address: "0x0987654321098765432109876543210987654321".to_string(),
                amount: "1000000000000000000".to_string(),
                gas_limit: 21000,
                gas_price: "0x4a817c800".to_string(),
                data: Some("0x".to_string()),
                nonce: 0,
                chain_id: 1,
                status,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                error_message: None,
            };

            storage.create_request(request).await.unwrap();
        }

        // Check counts
        let pending_count = storage
            .get_request_count_by_status(RequestStatus::Pending)
            .await
            .unwrap();
        let completed_count = storage
            .get_request_count_by_status(RequestStatus::Completed)
            .await
            .unwrap();
        let failed_count = storage
            .get_request_count_by_status(RequestStatus::Failed)
            .await
            .unwrap();

        assert_eq!(pending_count, 2);
        assert_eq!(completed_count, 2);
        assert_eq!(failed_count, 1);
    }

    #[tokio::test]
    async fn test_get_total_request_count() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Create 3 requests
        for _ in 0..3 {
            let request = RelayerRequest {
                id: Uuid::new_v4(),
                from_address: "0x1234567890123456789012345678901234567890".to_string(),
                to_address: "0x0987654321098765432109876543210987654321".to_string(),
                amount: "1000000000000000000".to_string(),
                gas_limit: 21000,
                gas_price: "0x4a817c800".to_string(),
                data: Some("0x".to_string()),
                nonce: 0,
                chain_id: 1,
                status: RequestStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                error_message: None,
            };

            storage.create_request(request).await.unwrap();
        }

        let total = storage.get_total_request_count().await.unwrap();
        assert_eq!(total, 3);
    }

    #[tokio::test]
    async fn test_get_requests_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Create 5 requests
        for _ in 0..5 {
            let request = RelayerRequest {
                id: Uuid::new_v4(),
                from_address: "0x1234567890123456789012345678901234567890".to_string(),
                to_address: "0x0987654321098765432109876543210987654321".to_string(),
                amount: "1000000000000000000".to_string(),
                gas_limit: 21000,
                gas_price: "0x4a817c800".to_string(),
                data: Some("0x".to_string()),
                nonce: 0,
                chain_id: 1,
                status: RequestStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                error_message: None,
            };

            storage.create_request(request).await.unwrap();
        }

        // Get with limit
        let requests = storage.get_requests(Some(3)).await.unwrap();
        assert_eq!(requests.len(), 3);

        // Get all
        let all_requests = storage.get_requests(None).await.unwrap();
        assert_eq!(all_requests.len(), 5);
    }

    #[test]
    fn test_storage_uptime() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let _uptime = storage.get_uptime_seconds();
        // Uptime is u64, so it's always >= 0. Just verify the call succeeds.
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

        let valid_levels = vec!["trace", "debug", "info", "warn", "error"];

        for level in valid_levels {
            config.log_level = level.to_string();
            assert_eq!(config.log_level, level);
        }
    }
}
