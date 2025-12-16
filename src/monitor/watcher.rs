use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::monitor::AppState;

/// Start watching the Claude directory for changes
pub async fn start_watching(
    state: Arc<RwLock<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let projects_dir = {
        let state = state.read().await;
        state.config.projects_dir.clone()
    };

    if !projects_dir.exists() {
        tracing::warn!("Projects directory does not exist: {:?}", projects_dir);
        return Ok(());
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(2)),
    )?;

    watcher.watch(&projects_dir, RecursiveMode::Recursive)?;

    tracing::info!("Started watching {:?}", projects_dir);

    // Keep watcher alive
    let _watcher = watcher;

    // Debounce refresh - wait for events to settle before refreshing
    let mut last_refresh = std::time::Instant::now();
    let debounce_duration = Duration::from_millis(500);

    loop {
        tokio::select! {
            Some(_event) = rx.recv() => {
                let now = std::time::Instant::now();
                if now.duration_since(last_refresh) > debounce_duration {
                    last_refresh = now;

                    // Delay a bit to let file writes complete
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    let mut state = state.write().await;
                    if let Err(e) = state.refresh().await {
                        tracing::error!("Failed to refresh data: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Stopping file watcher");
                break;
            }
        }
    }

    Ok(())
}
