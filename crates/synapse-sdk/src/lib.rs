use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SynapseClientError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Request error: {0}")]
    Request(String),
    #[error("Query error: {query_name}: {message}")]
    Query { query_name: String, message: String },
}

pub type Result<T> = std::result::Result<T, SynapseClientError>;

/// Synapse client — connects to a running Synapse runtime over HTTP.
///
/// ```rust,no_run
/// use synapse_sdk::Client;
///
/// #[tokio::main]
/// async fn main() {
///     let client = Client::new("http://localhost:8080");
///     client.emit("save", serde_json::json!({"content": "hello"})).await.unwrap();
///     let results = client.query("GetAll", serde_json::json!({})).await.unwrap();
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub fn with_timeout(base_url: &str, timeout: std::time::Duration) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
        }
    }

    /// Emit an event to trigger a handler.
    pub async fn emit(
        &self,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/emit", self.base_url);
        let body = serde_json::json!({
            "event": event,
            "payload": payload,
        });

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapseClientError::Connection(e.to_string()))?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(SynapseClientError::Request(msg));
        }

        resp.json()
            .await
            .map_err(|e| SynapseClientError::Request(e.to_string()))
    }

    /// Execute a named query.
    pub async fn query(
        &self,
        query_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/query", self.base_url);
        let body = serde_json::json!({
            "query": query_name,
            "params": params,
        });

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SynapseClientError::Connection(e.to_string()))?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(SynapseClientError::Query {
                query_name: query_name.into(),
                message: msg,
            });
        }

        resp.json()
            .await
            .map_err(|e| SynapseClientError::Request(e.to_string()))
    }

    /// Health check.
    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| SynapseClientError::Connection(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| SynapseClientError::Request(e.to_string()))
    }

    /// Get runtime status.
    pub async fn status(&self) -> Result<StatusResponse> {
        let url = format!("{}/status", self.base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| SynapseClientError::Connection(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| SynapseClientError::Request(e.to_string()))
    }

    /// Simple connectivity check.
    pub async fn ping(&self) -> bool {
        self.health().await.is_ok()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub handlers: Vec<String>,
    pub queries: Vec<String>,
    pub memories: Vec<String>,
    pub uptime_secs: i64,
}
