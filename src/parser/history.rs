use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A history entry from history.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub display: Option<String>,
    pub timestamp: Option<i64>,
    pub project: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

impl HistoryEntry {
    pub fn timestamp_utc(&self) -> Option<DateTime<Utc>> {
        self.timestamp.map(|ts| {
            DateTime::from_timestamp_millis(ts).unwrap_or_else(|| Utc::now())
        })
    }
}

/// Parse the history.jsonl file
pub fn parse_history_file(path: &Path) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Get unique projects from history
pub fn get_unique_projects(entries: &[HistoryEntry]) -> Vec<String> {
    let mut projects: Vec<String> = entries
        .iter()
        .filter_map(|e| e.project.clone())
        .collect();

    projects.sort();
    projects.dedup();
    projects
}
