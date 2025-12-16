use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Path to Claude Code data directory
    pub claude_dir: PathBuf,
    /// Path to projects directory
    pub projects_dir: PathBuf,
    /// Path to history file
    pub history_file: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        let claude_dir = home.join(".claude");

        Self {
            projects_dir: claude_dir.join("projects"),
            history_file: claude_dir.join("history.jsonl"),
            claude_dir,
        }
    }
}
