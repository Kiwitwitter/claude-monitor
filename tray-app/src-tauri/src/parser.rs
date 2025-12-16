use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Rolling window duration in hours (Max plan = 5 hours)
const ROLLING_WINDOW_HOURS: i64 = 5;

/// Default token limit for Max plan (this is an estimate, adjust as needed)
/// Claude Max plan limit is approximately 45M tokens per 5-hour window
const DEFAULT_TOKEN_LIMIT: u64 = 45_000_000;

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

    /// Billable tokens (cache reads are usually free or discounted)
    pub fn billable(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_input_tokens
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

/// Token usage with timestamp for rolling window calculation
#[derive(Debug, Clone)]
struct TimestampedUsage {
    timestamp: DateTime<Utc>,
    usage: TokenUsage,
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

/// Budget information for the rolling window
#[derive(Debug, Clone, Default, Serialize)]
pub struct BudgetInfo {
    /// Token limit for the rolling window
    pub limit: u64,
    /// Tokens used in the rolling window
    pub used: u64,
    /// Remaining tokens
    pub remaining: u64,
    /// Usage percentage (0-100)
    pub percentage: f64,
    /// Rolling window duration in hours
    pub window_hours: i64,
    /// Time until oldest tokens expire (in minutes)
    pub reset_minutes: Option<i64>,
}

impl BudgetInfo {
    pub fn new(used: u64, limit: u64, oldest_timestamp: Option<DateTime<Utc>>) -> Self {
        let remaining = limit.saturating_sub(used);
        let percentage = if limit > 0 {
            (used as f64 / limit as f64) * 100.0
        } else {
            0.0
        };

        let reset_minutes = oldest_timestamp.map(|ts| {
            let expiry = ts + Duration::hours(ROLLING_WINDOW_HOURS);
            let now = Utc::now();
            if expiry > now {
                (expiry - now).num_minutes()
            } else {
                0
            }
        });

        Self {
            limit,
            used,
            remaining,
            percentage,
            window_hours: ROLLING_WINDOW_HOURS,
            reset_minutes,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub total_usage: TokenUsage,
    pub rolling_usage: TokenUsage,
    pub budget: BudgetInfo,
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

/// Parse session file and return both total usage and timestamped usage entries
fn parse_session_file(path: &Path) -> Option<(SessionData, Vec<TimestampedUsage>)> {
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
    let mut timestamped_usages: Vec<TimestampedUsage> = Vec::new();

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

        // Parse timestamp
        let timestamp = entry.timestamp.as_ref().and_then(|ts| {
            DateTime::parse_from_rfc3339(ts).ok().map(|dt| dt.with_timezone(&Utc))
        });

        if let Some(msg) = entry.message {
            if let Some(msg_usage) = msg.usage {
                usage += msg_usage.clone();

                // Store timestamped usage for rolling window calculation
                if let Some(ts) = timestamp {
                    timestamped_usages.push(TimestampedUsage {
                        timestamp: ts,
                        usage: msg_usage,
                    });
                }
            }
        }

        if let Some(ts) = timestamp {
            if last_timestamp.map(|lt| ts > lt).unwrap_or(true) {
                last_timestamp = Some(ts);
            }
        }
    }

    Some((
        SessionData {
            session_id,
            project_path,
            usage,
            message_count,
            last_activity: last_timestamp,
            is_agent,
        },
        timestamped_usages,
    ))
}

pub fn get_stats() -> Result<Stats, Box<dyn std::error::Error>> {
    let claude_dir = get_claude_dir().ok_or("Could not find Claude directory")?;
    let projects_dir = claude_dir.join("projects");

    if !projects_dir.exists() {
        return Ok(Stats::default());
    }

    let mut sessions: Vec<SessionData> = Vec::new();
    let mut all_timestamped_usages: Vec<TimestampedUsage> = Vec::new();

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

            if let Some((session_data, timestamped_usages)) = parse_session_file(&session_path) {
                sessions.push(session_data);
                all_timestamped_usages.extend(timestamped_usages);
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
    let window_start = now - Duration::hours(ROLLING_WINDOW_HOURS);

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

    // Calculate rolling window usage
    let mut rolling_usage = TokenUsage::default();
    let mut oldest_in_window: Option<DateTime<Utc>> = None;

    for tu in &all_timestamped_usages {
        if tu.timestamp >= window_start {
            rolling_usage += tu.usage.clone();
            if oldest_in_window.map(|o| tu.timestamp < o).unwrap_or(true) {
                oldest_in_window = Some(tu.timestamp);
            }
        }
    }

    // Create budget info
    let budget = BudgetInfo::new(rolling_usage.billable(), DEFAULT_TOKEN_LIMIT, oldest_in_window);

    let mut projects: Vec<ProjectStats> = project_map.into_values().collect();
    projects.sort_by(|a, b| b.usage.total().cmp(&a.usage.total()));

    Ok(Stats {
        total_usage,
        rolling_usage,
        budget,
        active_sessions,
        active_agents,
        total_messages,
        projects,
    })
}
