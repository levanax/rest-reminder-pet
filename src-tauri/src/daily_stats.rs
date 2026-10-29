use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::config::{daily_stats_path, ensure_dirs};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPayload {
    pub success_count: u32,
    pub reminder_count: u32,
    pub this_delay_secs: u64,
    pub delay_total_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RestOutcome {
    Success,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestEvent {
    /// Local ISO-like datetime, e.g. 2026-10-28T14:32:05
    pub at: String,
    pub outcome: RestOutcome,
    pub delay_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRestStats {
    pub date: String,
    pub success_count: u32,
    pub incomplete_count: u32,
    pub delay_total_secs: u64,
    #[serde(default)]
    pub events: Vec<RestEvent>,
}

fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn now_local_iso() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

impl DailyRestStats {
    pub fn empty_today() -> Self {
        Self {
            date: today(),
            success_count: 0,
            incomplete_count: 0,
            delay_total_secs: 0,
            events: Vec::new(),
        }
    }

    pub fn load() -> Self {
        let mut stats = if let Ok(text) = fs::read_to_string(daily_stats_path()) {
            serde_json::from_str::<DailyRestStats>(&text).unwrap_or_else(|_| Self::empty_today())
        } else {
            Self::empty_today()
        };
        stats.ensure_today();
        stats
    }

    pub fn ensure_today(&mut self) {
        let today = today();
        if self.date != today {
            self.date = today;
            self.success_count = 0;
            self.incomplete_count = 0;
            self.delay_total_secs = 0;
            self.events.clear();
            let _ = self.save();
        }
    }

    pub fn save(&self) -> Result<(), String> {
        ensure_dirs()?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(daily_stats_path(), text).map_err(|e| e.to_string())
    }

    fn push_event(&mut self, outcome: RestOutcome, delay_secs: u64) {
        self.events.push(RestEvent {
            at: now_local_iso(),
            outcome,
            delay_secs,
        });
    }

    pub fn record_success(&mut self, delay_secs: u64) -> SummaryPayload {
        self.ensure_today();
        self.success_count = self.success_count.saturating_add(1);
        self.delay_total_secs = self.delay_total_secs.saturating_add(delay_secs);
        self.push_event(RestOutcome::Success, delay_secs);
        let _ = self.save();
        SummaryPayload {
            success_count: self.success_count,
            reminder_count: self.success_count.saturating_add(self.incomplete_count),
            this_delay_secs: delay_secs,
            delay_total_secs: self.delay_total_secs,
        }
    }

    pub fn record_incomplete(&mut self, delay_secs: u64) {
        self.ensure_today();
        self.incomplete_count = self.incomplete_count.saturating_add(1);
        self.push_event(RestOutcome::Incomplete, delay_secs);
        let _ = self.save();
    }
}
