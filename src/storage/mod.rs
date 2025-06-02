use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::ping::{Endpoint, PingResult};

#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub endpoint: Endpoint,
    pub last_status: Option<bool>,
    pub uptime_percentage: f64,
    pub last_ping: Option<DateTime<Utc>>,
    pub avg_latency: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct MemoryStorage {
    pub endpoints: Arc<Mutex<HashMap<Uuid, Endpoint>>>,
    pub ping_results: Arc<Mutex<Vec<PingResult>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(Mutex::new(HashMap::new())),
            ping_results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_endpoint(&self, endpoint: &Endpoint) -> Result<()> {
        let mut endpoints = self.endpoints.lock().unwrap();
        endpoints.insert(endpoint.id, endpoint.clone());
        Ok(())
    }

    pub fn save_result(&self, result: &PingResult) -> Result<()> {
        let mut ping_results = self.ping_results.lock().unwrap();
        ping_results.push(result.clone());

        if ping_results.len() > 50000 {
            ping_results.drain(0..10000);
        }

        Ok(())
    }

    pub fn get_endpoint_stats(&self) -> Result<Vec<EndpointStats>> {
        let endpoints = self.endpoints.lock().unwrap();
        let ping_results = self.ping_results.lock().unwrap();

        let mut stats = Vec::new();

        for (_, endpoint) in endpoints.iter() {
            let endpoint_results: Vec<&PingResult> = ping_results
                .iter()
                .filter(|r| r.endpoint_id == endpoint.id)
                .collect();

            let last_status = endpoint_results.last().map(|r| r.status);
            let last_ping = endpoint_results.last().map(|r| r.timestamp);

            let avg_latency = if !endpoint_results.is_empty() {
                let total_latency: u64 = endpoint_results.iter().map(|r| r.latency_ms).sum();
                Some(total_latency / endpoint_results.len() as u64)
            } else {
                None
            };

            let uptime_percentage = if !endpoint_results.is_empty() {
                let successful_pings = endpoint_results.iter().filter(|r| r.status).count();
                (successful_pings as f64 / endpoint_results.len() as f64) * 100.0
            } else {
                0.0
            };

            stats.push(EndpointStats {
                endpoint: endpoint.clone(),
                last_status,
                uptime_percentage,
                last_ping,
                avg_latency,
            });
        }

        Ok(stats)
    }

    pub fn get_uptime_history(
        &self,
        endpoint_id: Uuid,
        hours: i64,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        let ping_results = self.ping_results.lock().unwrap();
        let since = Utc::now() - Duration::hours(hours);

        let endpoint_results: Vec<&PingResult> = ping_results
            .iter()
            .filter(|r| r.endpoint_id == endpoint_id && r.timestamp >= since)
            .collect();

        if endpoint_results.is_empty() {
            return Ok(vec![]);
        }

        let mut hourly_groups: HashMap<i64, Vec<&PingResult>> = HashMap::new();

        for result in endpoint_results {
            let hour_key = result.timestamp.timestamp() / 3600;
            hourly_groups
                .entry(hour_key)
                .or_insert_with(Vec::new)
                .push(result);
        }

        let mut history = Vec::new();
        let start_hour = (since.timestamp() / 3600) * 3600;
        let end_hour = (Utc::now().timestamp() / 3600) * 3600;

        for hour in (start_hour..=end_hour).step_by(3600) {
            let hour_key = hour / 3600;
            let uptime = if let Some(results) = hourly_groups.get(&hour_key) {
                let successful = results.iter().filter(|r| r.status).count();
                (successful as f64 / results.len() as f64) * 100.0
            } else {
                0.0
            };

            let timestamp = DateTime::from_timestamp(hour, 0)
                .unwrap_or_else(|| Utc::now())
                .with_timezone(&Utc);

            history.push((timestamp, uptime));
        }

        history.sort_by_key(|(timestamp, _)| *timestamp);
        Ok(history)
    }
}
