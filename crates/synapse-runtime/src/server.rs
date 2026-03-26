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
        .route("/inspect", get(inspect))
        .route("/emit", post(emit))
        .route("/query", post(query))
        .route("/reload", post(reload))
        .route("/clear", post(clear))
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
    channels: Vec<String>,
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
        handlers: rt
            .handler_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        queries: rt
            .query_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        memories: rt
            .memory_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        channels: rt
            .channel_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        uptime_secs: uptime,
    })
}

async fn emit(
    State(state): State<AppState>,
    Json(req): Json<EmitRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("emit {} {}", req.event, req.payload);
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
    tracing::info!("query {} {}", req.query, req.params);
    let rt = state.read().await;
    match rt.query(&req.query, req.params).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn inspect(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.read().await;
    let names: Vec<&str> = rt.memory_names();
    let data = rt.storage.inspect(&names).await;
    Json(data)
}

async fn clear(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("clear all databases requested");
    let rt = state.read().await;
    let names: Vec<&str> = rt.memory_names();
    match rt.storage.clear(&names).await {
        Ok(report) => Ok(Json(serde_json::json!({
            "success": true,
            "cleared": report,
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn reload(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("reload requested");
    let mut rt = state.write().await;
    match rt.reload() {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "runtime reloaded"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
