use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::app::{LogEntry, LogLevel};
use crate::ping::{Endpoint, PingResult};
use crate::storage::{EndpointStats, MemoryStorage};

#[allow(async_fn_in_trait)]
pub trait Visitor {
    async fn visit_endpoint(&mut self, endpoint: &Endpoint) -> Result<()>;
}

pub struct PollingVisitor {
    client: Client,
    result_sender: mpsc::UnboundedSender<PingResult>,
    log_sender: mpsc::UnboundedSender<LogEntry>,
}

impl PollingVisitor {
    pub fn new(
        result_sender: mpsc::UnboundedSender<PingResult>,
        log_sender: mpsc::UnboundedSender<LogEntry>,
    ) -> Self {
        let client = Client::builder()
            .timeout(StdDuration::from_secs(10))
            .user_agent("pingme/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            result_sender,
            log_sender,
        }
    }

    fn send_log(&self, level: LogLevel, message: String) {
        let _ = self.log_sender.send(LogEntry {
            timestamp: Utc::now(),
            level,
            message,
        });
    }

    async fn ping_endpoint(&self, endpoint: &Endpoint) -> Result<PingResult> {
        let start = Instant::now();

        let url = if endpoint.url.starts_with("http://") || endpoint.url.starts_with("https://") {
            endpoint.url.clone()
        } else {
            format!("http://{}", endpoint.url)
        };

        self.send_log(LogLevel::Info, format!("Pinging: {}", url));

        //try with the head method first it's faster
        let methods = [
            |client: &Client, url: &str| client.head(url),
            |client: &Client, url: &str| client.get(url),
        ];

        for (attempt, method) in methods.iter().enumerate() {
            match method(&self.client, &url)
                .timeout(StdDuration::from_secs(5))
                .send()
                .await
            {
                Ok(response) => {
                    let latency = start.elapsed().as_millis() as u64;
                    let status = response.status().is_success();

                    if status {
                        self.send_log(LogLevel::Success, format!("{} - UP ({}ms)", url, latency));
                    } else {
                        self.send_log(
                            LogLevel::Warning,
                            format!("{} - Status: {} ({}ms)", url, response.status(), latency),
                        );
                    }

                    return Ok(PingResult {
                        endpoint_id: endpoint.id,
                        latency_ms: latency,
                        status,
                        timestamp: Utc::now(),
                    });
                }
                Err(e) => {
                    if attempt == 0 && e.is_timeout() {
                        continue;
                    }
                    if attempt == methods.len() - 1 {
                        let latency = start.elapsed().as_millis() as u64;
                        return Ok(PingResult {
                            endpoint_id: endpoint.id,
                            status: false,
                            latency_ms: latency,
                            timestamp: Utc::now(),
                        });
                    }
                }
            }
        }

        let latency = start.elapsed().as_millis() as u64;
        self.send_log(LogLevel::Error, format!("{} - DOWN: {}ms", url, latency));
        Ok(PingResult {
            endpoint_id: endpoint.id,
            status: false,
            latency_ms: latency,
            timestamp: Utc::now(),
        })
    }
}

impl Visitor for PollingVisitor {
    async fn visit_endpoint(&mut self, endpoint: &Endpoint) -> Result<()> {
        let result = self.ping_endpoint(endpoint).await?;
        self.result_sender
            .send(result)
            .context("Failed to send ping result")?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct StorageVisitor {
    pub storage: MemoryStorage,
}

impl StorageVisitor {
    pub fn new() -> Self {
        Self {
            storage: MemoryStorage::new(),
        }
    }

    pub fn add_endpoint(&self, endpoint: &Endpoint) -> Result<()> {
        self.storage.add_endpoint(endpoint)
    }

    pub fn save_result(&self, result: &PingResult) -> Result<()> {
        self.storage.save_result(result)
    }

    pub fn get_endpoint_stats(&self) -> Result<Vec<EndpointStats>> {
        self.storage.get_endpoint_stats()
    }

    pub fn get_uptime_history(
        &self,
        endpoint_id: Uuid,
        hours: i64,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        self.storage.get_uptime_history(endpoint_id, hours)
    }
}
