use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

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
    pub fn total(&self) -> u64 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
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

#[derive(Debug, Clone, Deserialize)]
struct MessageEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    message: Option<Message>,
    timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionData {
    pub session_id: String,
    pub project_path: String,
    pub usage: TokenUsage,
    pub message_count: u32,
    pub last_activity: Option<DateTime<Utc>>,
    pub is_agent: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub total_usage: TokenUsage,
    pub active_sessions: u32,
    pub active_agents: u32,
    pub total_messages: u32,
    pub projects: Vec<ProjectStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStats {
    pub path: String,
    pub usage: TokenUsage,
    pub session_count: u32,
    pub message_count: u32,
}

impl ProjectStats {
    pub fn new(path: String) -> Self {
        Self {
            path,
            usage: TokenUsage::default(),
            session_count: 0,
            message_count: 0,
        }
    }
}

fn get_claude_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

fn parse_session_file(path: &Path) -> Option<SessionData> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let file_name = path.file_stem()?.to_str()?;
    let is_agent = file_name.starts_with("agent-");

    let session_id = if is_agent {
        file_name.strip_prefix("agent-").unwrap_or(file_name)
    } else {
        file_name
    }
    .to_string();

    let project_path = path
        .parent()?
        .file_name()?
        .to_str()
        .map(|s| s.replace('-', "/"))
        .unwrap_or_default();

    let mut usage = TokenUsage::default();
    let mut message_count = 0u32;
    let mut last_timestamp: Option<DateTime<Utc>> = None;

    for line in reader.lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }

        let entry: MessageEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.entry_type.as_deref() == Some("assistant")
            || entry.entry_type.as_deref() == Some("user")
        {
            message_count += 1;
        }

        if let Some(msg) = entry.message {
            if let Some(msg_usage) = msg.usage {
                usage += msg_usage;
            }
        }

        if let Some(ts) = entry.timestamp {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(&ts) {
                let utc: DateTime<Utc> = parsed.into();
                if last_timestamp.map(|lt| utc > lt).unwrap_or(true) {
                    last_timestamp = Some(utc);
                }
            }
        }
    }

    Some(SessionData {
        session_id,
        project_path,
        usage,
        message_count,
        last_activity: last_timestamp,
        is_agent,
    })
}

pub fn get_stats() -> Result<Stats, Box<dyn std::error::Error>> {
    let claude_dir = get_claude_dir().ok_or("Could not find Claude directory")?;
    let projects_dir = claude_dir.join("projects");

    if !projects_dir.exists() {
        return Ok(Stats::default());
    }

    let mut sessions: Vec<SessionData> = Vec::new();

    for project_entry in fs::read_dir(&projects_dir)? {
        let project_entry = project_entry?;
        let project_path = project_entry.path();

        if !project_path.is_dir() {
            continue;
        }

        for session_entry in fs::read_dir(&project_path)? {
            let session_entry = session_entry?;
            let session_path = session_entry.path();

            if session_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }

            if let Some(session_data) = parse_session_file(&session_path) {
                sessions.push(session_data);
            }
        }
    }

    // Calculate stats
    let mut total_usage = TokenUsage::default();
    let mut active_sessions = 0u32;
    let mut active_agents = 0u32;
    let mut total_messages = 0u32;
    let mut project_map: HashMap<String, ProjectStats> = HashMap::new();

    let now = Utc::now();

    for session in &sessions {
        total_usage += session.usage.clone();
        total_messages += session.message_count;

        let is_active = session
            .last_activity
            .map(|la| (now - la).num_seconds() < 300)
            .unwrap_or(false);

        if is_active {
            if session.is_agent {
                active_agents += 1;
            } else {
                active_sessions += 1;
            }
        }

        let entry = project_map
            .entry(session.project_path.clone())
            .or_insert_with(|| ProjectStats::new(session.project_path.clone()));
        entry.usage += session.usage.clone();
        entry.session_count += 1;
        entry.message_count += session.message_count;
    }

    let mut projects: Vec<ProjectStats> = project_map.into_values().collect();
    projects.sort_by(|a, b| b.usage.total().cmp(&a.usage.total()));

    Ok(Stats {
        total_usage,
        active_sessions,
        active_agents,
        total_messages,
        projects,
    })
}
