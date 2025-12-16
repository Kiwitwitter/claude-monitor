use crate::config::Config;
use crate::parser::{self, SessionData, TokenUsage};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

/// Application state holding all monitoring data
#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub sessions: HashMap<String, SessionData>,
    pub last_refresh: Option<DateTime<Utc>>,
}

/// Summary statistics for the dashboard
#[derive(Debug, Clone, Serialize)]
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

impl AppState {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            sessions: HashMap::new(),
            last_refresh: None,
        }
    }

    /// Refresh all data from disk
    pub async fn refresh(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.sessions.clear();

        // Read all project directories
        if self.config.projects_dir.exists() {
            for project_entry in fs::read_dir(&self.config.projects_dir)? {
                let project_entry = project_entry?;
                let project_path = project_entry.path();

                if !project_path.is_dir() {
                    continue;
                }

                // Read all session files in this project
                for session_entry in fs::read_dir(&project_path)? {
                    let session_entry = session_entry?;
                    let session_path = session_entry.path();

                    // Only process .jsonl files
                    if session_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                        continue;
                    }

                    match parser::session::parse_session_file(&session_path) {
                        Ok(session_data) => {
                            let key = format!(
                                "{}:{}",
                                session_data.project_path, session_data.session_id
                            );
                            self.sessions.insert(key, session_data);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse session file {:?}: {}",
                                session_path,
                                e
                            );
                        }
                    }
                }
            }
        }

        self.last_refresh = Some(Utc::now());
        tracing::info!("Refreshed data: {} sessions loaded", self.sessions.len());
        Ok(())
    }

    /// Get aggregated statistics
    pub fn get_stats(&self) -> Stats {
        let mut total_usage = TokenUsage::default();
        let mut active_sessions = 0u32;
        let mut active_agents = 0u32;
        let mut total_messages = 0u32;
        let mut project_map: HashMap<String, (TokenUsage, u32, u32)> = HashMap::new();

        let now = Utc::now();

        for session in self.sessions.values() {
            total_usage += session.usage.clone();
            total_messages += session.message_count;

            // Check if session is active (last activity within 5 minutes)
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

            // Aggregate by project
            let entry = project_map.entry(session.project_path.clone()).or_default();
            entry.0 += session.usage.clone();
            entry.1 += 1;
            entry.2 += session.message_count;
        }

        let mut projects: Vec<ProjectStats> = project_map
            .into_iter()
            .map(|(path, (usage, session_count, message_count))| ProjectStats {
                path,
                usage,
                session_count,
                message_count,
            })
            .collect();

        // Sort by total tokens descending
        projects.sort_by(|a, b| b.usage.total().cmp(&a.usage.total()));

        Stats {
            total_usage,
            active_sessions,
            active_agents,
            total_messages,
            projects,
        }
    }

    /// Get list of active sessions
    pub fn get_active_sessions(&self) -> Vec<&SessionData> {
        let now = Utc::now();

        let mut sessions: Vec<_> = self
            .sessions
            .values()
            .filter(|s| {
                s.last_activity
                    .map(|la| (now - la).num_seconds() < 300)
                    .unwrap_or(false)
            })
            .collect();

        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        sessions
    }
}
