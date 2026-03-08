use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::interpreter::Runtime;

type AppState = Arc<RwLock<Runtime>>;

/// Build the HTTP router for the synapse runtime server.
pub fn build_router(runtime: Runtime) -> Router {
    let state: AppState = Arc::new(RwLock::new(runtime));

    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/emit", post(emit))
        .route("/query", post(query))
        .with_state(state)
}

/// Start the server on the given address.
pub async fn serve(runtime: Runtime, addr: &str) -> anyhow::Result<()> {
    let router = build_router(runtime);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Synapse runtime listening on {addr}");

    axum::serve(listener, router).await?;
    Ok(())
}

// ─── Request/Response types ──────────────────────────────────

#[derive(Deserialize)]
struct EmitRequest {
    event: String,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct EmitResponse {
    success: bool,
    message: String,
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    params: serde_json::Value,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    uptime_secs: i64,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    handlers: Vec<String>,
    queries: Vec<String>,
    memories: Vec<String>,
    uptime_secs: i64,
}

// ─── Handlers ────────────────────────────────────────────────

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.read().await;
    let stats = rt.stats.read().await;
    let uptime = stats
        .started_at
        .map(|s| (chrono::Utc::now() - s).num_seconds())
        .unwrap_or(0);

    Json(HealthResponse {
        status: "healthy".into(),
        uptime_secs: uptime,
    })
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.read().await;
    let stats = rt.stats.read().await;
    let uptime = stats
        .started_at
        .map(|s| (chrono::Utc::now() - s).num_seconds())
        .unwrap_or(0);

    Json(StatusResponse {
        status: "running".into(),
        handlers: rt.handler_names().into_iter().map(|s| s.to_string()).collect(),
        queries: rt.query_names().into_iter().map(|s| s.to_string()).collect(),
        memories: rt.memory_names().into_iter().map(|s| s.to_string()).collect(),
        uptime_secs: uptime,
    })
}

async fn emit(
    State(state): State<AppState>,
    Json(req): Json<EmitRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.read().await;
    match rt.emit(&req.event, req.payload).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.read().await;
    match rt.query(&req.query, req.params).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}
