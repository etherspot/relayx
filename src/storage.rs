use crate::types::{RelayerRequest, RelayerResponse, RequestStatus};
use anyhow::Result;
use rocksdb::{DBWithThreadMode, MultiThreaded, Options};
use serde_json;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub struct Storage {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
    start_time: std::time::Instant,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(10000);
        opts.set_use_fsync(false);
        opts.set_bytes_per_sync(1024 * 1024);

        let db = DBWithThreadMode::<MultiThreaded>::open(&opts, path)?;

        Ok(Self {
            db: Arc::new(db),
            start_time: std::time::Instant::now(),
        })
    }

    /// Store a new relayer request
    pub async fn store_request(&self, request: &RelayerRequest) -> Result<()> {
        let key = format!("request:{}", request.id);
        let value = serde_json::to_string(request)?;

        self.db.put(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    /// Retrieve a relayer request by ID
    pub async fn get_request(&self, id: Uuid) -> Result<Option<RelayerRequest>> {
        let key = format!("request:{}", id);

        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let request: RelayerRequest = serde_json::from_slice(&value)?;
                Ok(Some(request))
            }
            None => Ok(None),
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
        if let Some(mut request) = self.get_request(id).await? {
            request.status = status;
            request.updated_at = chrono::Utc::now();
            request.error_message = error_message;
            self.store_request(&request).await?;
        }
        Ok(())
    }

    /// Get all requests with optional filtering
    pub async fn get_requests(&self, limit: Option<usize>) -> Result<Vec<RelayerRequest>> {
        let mut requests = Vec::new();
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

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
            }
        }

        Ok(requests)
    }

    /// Get request count by status
    pub async fn get_request_count_by_status(&self, status: RequestStatus) -> Result<u64> {
        let mut count = 0;
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for result in iter {
            let (_, value) = result?;

            if let Ok(request) = serde_json::from_slice::<RelayerRequest>(&value) {
                if request.status == status {
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Get total request count
    pub async fn get_total_request_count(&self) -> Result<u64> {
        let mut count = 0;
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for result in iter {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with("request:") {
                count += 1;
            }
        }

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
