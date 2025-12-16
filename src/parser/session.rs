use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Token usage data from a Claude Code message
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    pub fn total_input(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }

    pub fn total(&self) -> u64 {
        self.total_input() + self.output_tokens
    }
}

impl std::ops::Add for TokenUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens
                + other.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens + other.cache_read_input_tokens,
        }
    }
}

impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, other: Self) {
        *self = self.clone() + other;
    }
}

/// A message entry in a session
#[derive(Debug, Clone, Deserialize)]
pub struct MessageEntry {
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub message: Option<Message>,
    pub timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub role: Option<String>,
    pub usage: Option<TokenUsage>,
    pub model: Option<String>,
}

/// Aggregated session data
#[derive(Debug, Clone, Serialize)]
pub struct SessionData {
    pub session_id: String,
    pub project_path: String,
    pub usage: TokenUsage,
    pub message_count: u32,
    pub last_activity: Option<DateTime<Utc>>,
    pub is_agent: bool,
}

/// Parse a session JSONL file
pub fn parse_session_file(path: &Path) -> Result<SessionData, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let is_agent = file_name.starts_with("agent-");

    let session_id = if is_agent {
        file_name.strip_prefix("agent-").unwrap_or(file_name)
    } else {
        file_name
    }
    .to_string();

    // Get project path from parent directory name
    let project_path = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(|s| s.replace('-', "/"))
        .unwrap_or_default();

    let mut usage = TokenUsage::default();
    let mut message_count = 0u32;
    let mut last_timestamp: Option<DateTime<Utc>> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        let entry: MessageEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Count messages
        if entry.entry_type.as_deref() == Some("assistant")
            || entry.entry_type.as_deref() == Some("user")
        {
            message_count += 1;
        }

        // Extract usage from assistant messages
        if let Some(msg) = entry.message {
            if let Some(msg_usage) = msg.usage {
                usage += msg_usage;
            }
        }

        // Track last timestamp
        if let Some(ts) = entry.timestamp {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(&ts) {
                let utc: DateTime<Utc> = parsed.into();
                if last_timestamp.map(|lt| utc > lt).unwrap_or(true) {
                    last_timestamp = Some(utc);
                }
            }
        }
    }

    Ok(SessionData {
        session_id,
        project_path,
        usage,
        message_count,
        last_activity: last_timestamp,
        is_agent,
    })
}

/// Check if a session is currently active (modified within last 5 minutes)
pub fn is_session_active(path: &Path) -> bool {
    if let Ok(metadata) = path.metadata() {
        if let Ok(modified) = metadata.modified() {
            let age = std::time::SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default();
            return age.as_secs() < 300; // 5 minutes
        }
    }
    false
}
