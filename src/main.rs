mod api;
mod config;
mod monitor;
mod parser;
mod web;

use clap::{Parser, Subcommand};
use std::process;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::monitor::state::AppState;

#[derive(Parser)]
#[command(name = "claude-monitor")]
#[command(about = "Monitor Claude Code usage with a local web dashboard")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the monitor server
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "3456")]
        port: u16,
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the monitor server
    Stop,
    /// Show current status
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start { port, foreground }) => {
            if !foreground {
                // Check if already running
                if is_running() {
                    eprintln!("claude-monitor is already running");
                    process::exit(1);
                }

                // For now, just run in foreground
                // TODO: Implement proper daemonization
                println!("Starting claude-monitor on port {}...", port);
            }
            start_server(port).await;
        }
        Some(Commands::Stop) => {
            stop_server();
        }
        Some(Commands::Status) => {
            show_status();
        }
        None => {
            // Default: start in foreground
            start_server(3456).await;
        }
    }
}

async fn start_server(port: u16) {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "claude_monitor=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::default();

    // Initialize app state
    let state = Arc::new(RwLock::new(AppState::new(&config)));

    // Initial load of data
    {
        let mut state = state.write().await;
        if let Err(e) = state.refresh().await {
            tracing::error!("Failed to load initial data: {}", e);
        }
    }

    // Start file watcher
    let watcher_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = monitor::watcher::start_watching(watcher_state).await {
            tracing::error!("File watcher error: {}", e);
        }
    });

    // Build router
    let app = api::routes::create_router(state);

    // Start server
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Claude Monitor running at http://{}", addr);
    println!("\n  Claude Monitor is running!");
    println!("  Open http://localhost:{} in your browser\n", port);

    // Handle graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutting down...");
}

fn is_running() -> bool {
    // Check for PID file
    if let Some(pid_file) = get_pid_file() {
        if pid_file.exists() {
            if let Ok(pid) = std::fs::read_to_string(&pid_file) {
                let pid = pid.trim();
                // Check if process is running
                let output = std::process::Command::new("kill")
                    .args(["-0", pid])
                    .output();
                return output.map(|o| o.status.success()).unwrap_or(false);
            }
        }
    }
    false
}

fn stop_server() {
    if let Some(pid_file) = get_pid_file() {
        if pid_file.exists() {
            if let Ok(pid) = std::fs::read_to_string(&pid_file) {
                let pid = pid.trim();
                let _ = std::process::Command::new("kill")
                    .arg(pid)
                    .status();
                let _ = std::fs::remove_file(&pid_file);
                println!("Stopped claude-monitor");
                return;
            }
        }
    }
    println!("claude-monitor is not running");
}

fn show_status() {
    if is_running() {
        println!("claude-monitor is running");
    } else {
        println!("claude-monitor is not running");
    }
}

fn get_pid_file() -> Option<std::path::PathBuf> {
    dirs::runtime_dir()
        .or_else(|| dirs::cache_dir())
        .map(|d| d.join("claude-monitor.pid"))
}
