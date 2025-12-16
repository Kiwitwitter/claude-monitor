use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::monitor::{state::Stats, AppState};
use crate::web::templates;

type SharedState = Arc<RwLock<AppState>>;

/// Create the main router
pub fn create_router(state: SharedState) -> Router {
    Router::new()
        // Main page
        .route("/", get(index_handler))
        // API routes
        .route("/api/stats", get(stats_handler))
        .route("/api/sessions", get(sessions_handler))
        .route("/api/refresh", get(refresh_handler))
        // HTMX partials
        .route("/partials/budget", get(budget_partial_handler))
        .route("/partials/stats", get(stats_partial_handler))
        .route("/partials/sessions", get(sessions_partial_handler))
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}

/// Main dashboard page
async fn index_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    let stats = state.get_stats();
    let active_sessions = state.get_active_sessions();

    let html = templates::render_index(&stats, &active_sessions);
    Html(html)
}

/// API: Get current stats
async fn stats_handler(State(state): State<SharedState>) -> Json<Stats> {
    let state = state.read().await;
    Json(state.get_stats())
}

/// API: Get active sessions
async fn sessions_handler(
    State(state): State<SharedState>,
) -> Json<Vec<crate::parser::SessionData>> {
    let state = state.read().await;
    let sessions: Vec<_> = state.get_active_sessions().into_iter().cloned().collect();
    Json(sessions)
}

/// API: Force refresh
async fn refresh_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let mut state = state.write().await;
    match state.refresh().await {
        Ok(_) => (StatusCode::OK, "Refreshed"),
        Err(e) => {
            tracing::error!("Refresh failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Refresh failed")
        }
    }
}

/// HTMX partial: Budget section
async fn budget_partial_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    let stats = state.get_stats();
    Html(templates::render_budget_partial(&stats))
}

/// HTMX partial: Stats cards
async fn stats_partial_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    let stats = state.get_stats();
    Html(templates::render_stats_partial(&stats))
}

/// HTMX partial: Active sessions list
async fn sessions_partial_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    let sessions = state.get_active_sessions();
    Html(templates::render_sessions_partial(&sessions))
}
