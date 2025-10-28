use std::{path::Path, sync::Arc};

use anyhow::Result;
use rocksdb::{DBWithThreadMode, MultiThreaded, Options};
use serde_json;
use uuid::Uuid;

use crate::types::{RelayerRequest, RelayerResponse, RequestStatus, Resubmission};

pub struct Storage {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
    start_time: std::time::Instant,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        tracing::debug!("Opening RocksDB database at: {:?}", path.as_ref());

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(10000);
        opts.set_use_fsync(false);
        opts.set_bytes_per_sync(1024 * 1024);

        let db = DBWithThreadMode::<MultiThreaded>::open(&opts, path.as_ref()).map_err(|e| {
            tracing::error!("Failed to open RocksDB database: {}", e);
            e
        })?;

        tracing::debug!("RocksDB database opened successfully");

        Ok(Self {
            db: Arc::new(db),
            start_time: std::time::Instant::now(),
        })
    }

    /// Store a new relayer request
    pub async fn store_request(&self, request: &RelayerRequest) -> Result<()> {
        let key = format!("request:{}", request.id);
        tracing::trace!("Storing request with key: {}", key);

        let value = serde_json::to_string(request).map_err(|e| {
            tracing::error!("Failed to serialize request: {}", e);
            e
        })?;

        self.db.put(key.as_bytes(), value.as_bytes()).map_err(|e| {
            tracing::error!("Failed to store request {}: {}", request.id, e);
            e
        })?;

        tracing::trace!("Request {} stored successfully", request.id);
        Ok(())
    }

    /// Create and store a new relayer request
    pub async fn create_request(&self, request: RelayerRequest) -> Result<()> {
        tracing::debug!(
            "Creating request: {} - Status: {:?}, Chain: {}",
            request.id,
            request.status,
            request.chain_id
        );
        self.store_request(&request).await
    }

    /// Retrieve a relayer request by ID
    pub async fn get_request(&self, id: Uuid) -> Result<Option<RelayerRequest>> {
        let key = format!("request:{}", id);
        tracing::trace!("Retrieving request with key: {}", key);

        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let request: RelayerRequest = serde_json::from_slice(&value).map_err(|e| {
                    tracing::error!("Failed to deserialize request {}: {}", id, e);
                    e
                })?;
                tracing::trace!(
                    "Request {} retrieved successfully - Status: {:?}",
                    id,
                    request.status
                );
                Ok(Some(request))
            }
            None => {
                tracing::trace!("Request {} not found", id);
                Ok(None)
            }
        }
    }

    /// Store a relayer response
    pub async fn store_response(&self, response: &RelayerResponse) -> Result<()> {
        let key = format!("response:{}", response.request_id);
        let value = serde_json::to_string(response)?;

        self.db.put(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    /// Retrieve a relayer response by request ID
    pub async fn get_response(&self, request_id: Uuid) -> Result<Option<RelayerResponse>> {
        let key = format!("response:{}", request_id);

        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let response: RelayerResponse = serde_json::from_slice(&value)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Update request status
    pub async fn update_request_status(
        &self,
        id: Uuid,
        status: RequestStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        tracing::debug!("Updating request {} status to: {:?}", id, status);

        if let Some(mut request) = self.get_request(id).await? {
            let old_status = request.status.clone();
            request.status = status;
            request.updated_at = chrono::Utc::now();
            request.error_message = error_message.clone();

            self.store_request(&request).await?;

            tracing::info!(
                "Request {} status updated: {:?} -> {:?}",
                id,
                old_status,
                request.status
            );
            if let Some(err) = error_message {
                tracing::warn!("Request {} error: {}", id, err);
            }
        } else {
            tracing::warn!("Attempted to update non-existent request: {}", id);
        }
        Ok(())
    }

    /// Update request transaction hash
    pub async fn update_request_tx_hash(&self, id: Uuid, tx_hash: String) -> Result<()> {
        tracing::debug!("Updating request {} tx hash to: {}", id, tx_hash);

        if let Some(mut request) = self.get_request(id).await? {
            request.transaction_hash = Some(tx_hash);
            request.updated_at = chrono::Utc::now();
            self.store_request(&request).await?;
            tracing::info!("Request {} transaction hash stored", id);
        } else {
            tracing::warn!("Attempted to set tx hash for non-existent request: {}", id);
        }
        Ok(())
    }

    /// Update request nonce
    pub async fn update_request_nonce(&self, id: Uuid, nonce: u64) -> Result<()> {
        tracing::debug!("Updating request {} nonce to: {}", id, nonce);
        if let Some(mut request) = self.get_request(id).await? {
            request.nonce = nonce;
            request.updated_at = chrono::Utc::now();
            self.store_request(&request).await?;
            tracing::info!("Request {} nonce stored", id);
        } else {
            tracing::warn!("Attempted to set nonce for non-existent request: {}", id);
        }
        Ok(())
    }

    /// Record a resubmission attempt for a request
    pub async fn add_resubmission(&self, request_id: Uuid, resub: &Resubmission) -> Result<()> {
        let key = format!(
            "resubmission:{}:{}:{}",
            request_id, resub.chain_id, resub.transaction_hash
        );
        let value = serde_json::to_string(resub)?;
        self.db.put(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    /// Retrieve resubmissions for a request
    pub async fn get_resubmissions(&self, request_id: Uuid) -> Result<Vec<Resubmission>> {
        let mut items = Vec::new();
        let prefix = format!("resubmission:{}:", request_id);
        let iter = self.db.iterator(rocksdb::IteratorMode::From(
            prefix.as_bytes(),
            rocksdb::Direction::Forward,
        ));
        for result in iter {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(&key);
            if !key_str.starts_with(&prefix) {
                break;
            }
            if let Ok(resub) = serde_json::from_slice::<Resubmission>(&value) {
                items.push(resub);
            }
        }
        Ok(items)
    }

    /// Get all requests with optional filtering
    pub async fn get_requests(&self, limit: Option<usize>) -> Result<Vec<RelayerRequest>> {
        tracing::debug!("Retrieving requests with limit: {:?}", limit);

        let mut requests = Vec::new();
        let iter = self.db.iterator(rocksdb::IteratorMode::From(
            b"request:",
            rocksdb::Direction::Forward,
        ));

        for result in iter {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with("request:") {
                if let Ok(request) = serde_json::from_slice::<RelayerRequest>(&value) {
                    requests.push(request);

                    if let Some(limit) = limit {
                        if requests.len() >= limit {
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }

        tracing::debug!("Retrieved {} requests", requests.len());
        Ok(requests)
    }

    /// Get request count by status
    pub async fn get_request_count_by_status(&self, status: RequestStatus) -> Result<u64> {
        tracing::trace!("Counting requests with status: {:?}", status);

        let mut count = 0;
        let iter = self.db.iterator(rocksdb::IteratorMode::From(
            b"request:",
            rocksdb::Direction::Forward,
        ));

        for result in iter {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with("request:") {
                break;
            }

            if let Ok(request) = serde_json::from_slice::<RelayerRequest>(&value) {
                if request.status == status {
                    count += 1;
                }
            }
        }

        tracing::trace!("Found {} requests with status {:?}", count, status);
        Ok(count)
    }

    /// Get total request count
    pub async fn get_total_request_count(&self) -> Result<u64> {
        tracing::trace!("Counting total requests");

        let mut count = 0;
        let iter = self.db.iterator(rocksdb::IteratorMode::From(
            b"request:",
            rocksdb::Direction::Forward,
        ));

        for result in iter {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with("request:") {
                count += 1;
            } else {
                break;
            }
        }

        tracing::trace!("Total requests: {}", count);
        Ok(count)
    }

    /// Get uptime in seconds
    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Clone for Storage {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            start_time: self.start_time,
        }
    }
}
