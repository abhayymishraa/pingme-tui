use anyhow::Result;
use chrono::{DateTime, Utc};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use tui_textarea::TextArea;
use uuid::Uuid;

use crate::storage::EndpointStats;
use crate::visitor::StorageVisitor;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Error,
    Success,
    Warning,
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Adding,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeRange {
    Minutes(i64),
}

impl TimeRange {
    pub fn display_name(&self) -> String {
        match self {
            TimeRange::Minutes(n) => format!("{}m", n),
        }
    }

    pub fn get_duration_hours(&self) -> Option<i64> {
        match self {
            TimeRange::Minutes(n) => Some(n / 60),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UptimeBlock {
    pub timestamp: DateTime<Utc>,
    pub status: bool,
}

pub struct App {
    pub endpoints_stats: Vec<EndpointStats>,
    pub selected_endpoint: usize,
    pub table_state: TableState,
    pub input_mode: InputMode,
    pub url_input: TextArea<'static>,
    pub uptime_history: HashMap<Uuid, Vec<(f64, f64)>>,
    pub uptime_blocks: HashMap<Uuid, Vec<UptimeBlock>>,
    pub developer_mode: bool,
    pub logs: Vec<LogEntry>,
    pub log_scroll: usize,
    pub time_ranges: Vec<TimeRange>,
    pub selected_time_range: usize,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let time_ranges = vec![TimeRange::Minutes(60)];

        Self {
            endpoints_stats: Vec::new(),
            selected_endpoint: 0,
            table_state,
            input_mode: InputMode::Normal,
            url_input: TextArea::default(),
            uptime_history: HashMap::new(),
            uptime_blocks: HashMap::new(),
            developer_mode: false,
            logs: Vec::new(),
            log_scroll: 0,
            time_ranges,
            selected_time_range: 0,
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: String) {
        self.logs.push(LogEntry {
            timestamp: Utc::now(),
            level,
            message,
        });

        if self.logs.len() > 1000 {
            self.logs.drain(0..100);
        }
    }

    pub fn toggle_developer_mode(&mut self) {
        self.developer_mode = !self.developer_mode;
        self.log_scroll = 0;
    }

    pub fn scroll_logs_up(&mut self) {
        if self.log_scroll > 0 {
            self.log_scroll -= 1;
        }
    }

    pub fn scroll_logs_down(&mut self) {
        if self.log_scroll + 1 < self.logs.len() {
            self.log_scroll += 1;
        }
    }

    pub fn get_current_time_range(&self) -> &TimeRange {
        &self.time_ranges[self.selected_time_range]
    }

    pub fn update_stats(&mut self, storage: &StorageVisitor) -> Result<()> {
        self.endpoints_stats = storage.get_endpoint_stats()?;

        let current_time_range = self.get_current_time_range().clone();

        let endpoint_ids: Vec<Uuid> = self.endpoints_stats.iter().map(|s| s.endpoint.id).collect();

        for endpoint_id in endpoint_ids {
            let hours = current_time_range.get_duration_hours().unwrap_or(24);
            let history = storage.get_uptime_history(endpoint_id, hours)?;
            let chart_data: Vec<(f64, f64)> = if !history.is_empty() {
                let now = Utc::now();
                history
                    .into_iter()
                    .map(|(timestamp, uptime)| {
                        let hours_ago = (now - timestamp).num_minutes() as f64 / 60.0;
                        let x_pos = hours as f64 - hours_ago.max(0.0).min(hours as f64);
                        (x_pos, uptime)
                    })
                    .collect()
            } else {
                vec![(0.0, 0.0), (hours as f64, 0.0)]
            };

            self.uptime_history.insert(endpoint_id, chart_data);

            if !self.uptime_blocks.contains_key(&endpoint_id) {
                self.update_uptime_blocks(storage, endpoint_id, &current_time_range)?;
            }
        }

        Ok(())
    }

    pub fn add_realtime_block(&mut self, endpoint_id: Uuid, status: bool) {
        let now = Utc::now();
        let new_block = UptimeBlock {
            timestamp: now,
            status,
        };

        // Get or create the blocks vector for this endpoint
        let blocks = self
            .uptime_blocks
            .entry(endpoint_id)
            .or_insert_with(Vec::new);

        // Add the new block
        blocks.push(new_block);

        // Keep only the last 60 blocks (for 60-minute window)
        if blocks.len() > 60 {
            blocks.drain(0..blocks.len() - 60);
        }

        // Sort by timestamp to ensure proper order
        blocks.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    fn update_uptime_blocks(
        &mut self,
        storage: &StorageVisitor,
        endpoint_id: Uuid,
        time_range: &TimeRange,
    ) -> Result<()> {
        let blocks = match time_range {
            TimeRange::Minutes(minutes) => {
                self.generate_minute_blocks(storage, endpoint_id, *minutes)?
            }
        };

        self.uptime_blocks.insert(endpoint_id, blocks);
        Ok(())
    }

    fn generate_minute_blocks(
        &self,
        storage: &StorageVisitor,
        endpoint_id: Uuid,
        minutes: i64,
    ) -> Result<Vec<UptimeBlock>> {
        let history = storage.get_uptime_history(endpoint_id, minutes / 60 + 1)?;
        let mut blocks = Vec::new();

        let now = Utc::now();
        let start_time = now - chrono::Duration::minutes(minutes);

        for (timestamp, uptime) in &history {
            if *timestamp >= start_time {
                let minutes_ago = (now - *timestamp).num_minutes();
                if minutes_ago >= 0 && minutes_ago < minutes {
                    blocks.push(UptimeBlock {
                        timestamp: *timestamp,
                        status: *uptime > 0.0,
                    });
                }
            }
        }

        blocks.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(blocks)
    }

    pub fn next_endpoint(&mut self) {
        if !self.endpoints_stats.is_empty() {
            self.selected_endpoint = (self.selected_endpoint + 1) % self.endpoints_stats.len();
            self.table_state.select(Some(self.selected_endpoint));
        }
    }

    // pub fn populate_test_data(&mut self) {
    //     for stats in &self.endpoints_stats {
    //         let mut test_data = Vec::new();
    //         for i in 0..24 {
    //             let uptime = 85.0 + (i as f64 * 0.5) + (i as f64).sin() * 10.0;
    //             test_data.push((i as f64, uptime.max(0.0).min(100.0)));
    //         }
    //         self.uptime_history.insert(stats.endpoint.id, test_data);

    //         // Generate test uptime blocks
    //         let mut test_blocks = Vec::new();
    //         let now = Utc::now();
    //         for i in 0..24 {
    //             let timestamp = now - chrono::Duration::hours(24 - i);
    //             let uptime_percentage = 85.0 + (i as f64 * 0.5) + (i as f64).sin() * 10.0;
    //             test_blocks.push(UptimeBlock {
    //                 timestamp,
    //                 status: uptime_percentage > 50.0,
    //                 uptime_percentage: uptime_percentage.max(0.0).min(100.0),
    //             });
    //         }
    //         self.uptime_blocks.insert(stats.endpoint.id, test_blocks);
    //     }
    // }

    pub fn previous_endpoint(&mut self) {
        if !self.endpoints_stats.is_empty() {
            self.selected_endpoint = if self.selected_endpoint == 0 {
                self.endpoints_stats.len() - 1
            } else {
                self.selected_endpoint - 1
            };
            self.table_state.select(Some(self.selected_endpoint));
        }
    }
}
