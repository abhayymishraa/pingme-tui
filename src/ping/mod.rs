use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::app::LogEntry;
use crate::visitor::{PollingVisitor, StorageVisitor, Visitor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: Uuid,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct PingResult {
    pub endpoint_id: Uuid,
    pub status: bool,
    pub latency_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub struct PingManager {
    storage: StorageVisitor,
    interval: StdDuration,
}

impl PingManager {
    pub fn new(interval_seconds: u64) -> Self {
        let storage = StorageVisitor::new();
        Self {
            storage,
            interval: StdDuration::from_secs(interval_seconds),
        }
    }

    pub fn add_endpoint(&mut self, url: String) -> Result<()> {
        let endpoint = Endpoint {
            id: Uuid::new_v4(),
            url,
        };

        self.storage.add_endpoint(&endpoint)?;
        Ok(())
    }

    pub fn get_all_enpoints(&self) -> Result<Vec<Endpoint>> {
        let endpoints = self.storage.storage.endpoints.lock().unwrap();
        Ok(endpoints.values().cloned().collect())
    }

    pub async fn start_polling(
        &self,
        result_sender: mpsc::UnboundedSender<PingResult>,
        log_sender: mpsc::UnboundedSender<LogEntry>,
    ) {
        let mut interval = tokio::time::interval(self.interval);
        let mut polling_visitor = PollingVisitor::new(result_sender, log_sender);

        loop {
            interval.tick().await;

            match self.get_all_enpoints() {
                Ok(endpoints) => {
                    for endpoint in endpoints {
                        if let Err(e) = polling_visitor.visit_endpoint(&endpoint).await {
                            eprintln!("Error polling {}: {}", endpoint.url, e)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error getting endpoints: {}", e);
                }
            }
        }
    }

    pub fn get_storage(&self) -> StorageVisitor {
        self.storage.clone()
    }
}
