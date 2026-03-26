use std::sync::Arc;

use super::{QueryFilter, StorageError, StorageResult};
use crate::llm::EmbeddingClient;
use crate::value::{Record, Value};

/// Pinecone vector storage backend using REST API.
pub struct PineconeBackend {
    url: String,
    api_key: String,
    client: reqwest::Client,
    embedder: Option<Arc<EmbeddingClient>>,
}

impl std::fmt::Debug for PineconeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PineconeBackend")
            .field("url", &self.url)
            .finish()
    }
}

impl PineconeBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let api_key = std::env::var("PINECONE_API_KEY").unwrap_or_default();
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            api_key,
            client: reqwest::Client::new(),
            embedder: None,
        })
    }

    pub fn set_embedder(&mut self, embedder: Arc<EmbeddingClient>) {
        self.embedder = Some(embedder);
    }

    pub async fn ensure_table(
        &self,
        _type_name: &str,
        _fields: &[(String, String)],
        _indexes: &[String],
    ) -> StorageResult<()> {
        Ok(())
    }

    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        let content = record
            .fields
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if content.is_empty() {
            return Ok(());
        }
        if let Some(ref embedder) = self.embedder {
            let vector = embedder
                .embed(content)
                .await
                .map_err(|e| StorageError::Qdrant(format!("embedding failed: {e}")))?;
            self.store_with_vector(record, vector).await
        } else {
            Ok(())
        }
    }

    pub async fn store_with_vector(&self, record: &Record, vector: Vec<f32>) -> StorageResult<()> {
        let mut metadata = serde_json::Map::new();
        metadata.insert("_type".into(), record.type_name.clone().into());
        for (k, v) in &record.fields {
            match v {
                Value::String(s) => {
                    metadata.insert(k.clone(), s.clone().into());
                }
                Value::Int(n) => {
                    metadata.insert(k.clone(), (*n).into());
                }
                Value::Float(f) => {
                    metadata.insert(k.clone(), (*f).into());
                }
                Value::Bool(b) => {
                    metadata.insert(k.clone(), (*b).into());
                }
                _ => {}
            }
        }

        let body = serde_json::json!({
            "vectors": [{
                "id": record.id,
                "values": vector,
                "metadata": metadata,
            }],
            "namespace": record.type_name,
        });

        self.client
            .post(format!("{}/vectors/upsert", self.url))
            .header("Api-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone upsert failed: {e}")))?;

        Ok(())
    }

    pub async fn search_by_vector(
        &self,
        type_name: &str,
        vector: Vec<f32>,
        limit: usize,
        threshold: f64,
    ) -> StorageResult<Vec<(String, f32)>> {
        let body = serde_json::json!({
            "vector": vector,
            "topK": limit,
            "includeMetadata": true,
            "namespace": type_name,
        });

        let resp = self
            .client
            .post(format!("{}/query", self.url))
            .header("Api-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone parse failed: {e}")))?;

        let mut scored = Vec::new();
        if let Some(matches) = result.get("matches").and_then(|m| m.as_array()) {
            for m in matches {
                let id = m.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let score = m.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                if score as f64 >= threshold {
                    scored.push((id.to_string(), score));
                }
            }
        }
        Ok(scored)
    }

    pub async fn get(&self, _type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let body = serde_json::json!({
            "ids": [id],
        });

        let resp = self
            .client
            .get(format!("{}/vectors/fetch", self.url))
            .header("Api-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone fetch failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone parse failed: {e}")))?;

        if let Some(vectors) = result.get("vectors").and_then(|v| v.as_object()) {
            if let Some(vec_data) = vectors.get(id) {
                let type_name = vec_data
                    .get("metadata")
                    .and_then(|m| m.get("_type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("_unknown");
                let mut record = Record::new(type_name);
                record.id = id.to_string();
                if let Some(metadata) = vec_data.get("metadata").and_then(|m| m.as_object()) {
                    for (k, v) in metadata {
                        if k.starts_with('_') {
                            continue;
                        }
                        record
                            .fields
                            .insert(k.clone(), json_to_value(v));
                    }
                }
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    pub async fn query(
        &self,
        _type_name: &str,
        _filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        Ok(vec![])
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        self.store(record).await
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let body = serde_json::json!({
            "ids": [id],
            "namespace": type_name,
        });

        self.client
            .post(format!("{}/vectors/delete", self.url))
            .header("Api-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone delete failed: {e}")))?;

        Ok(())
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        let body = serde_json::json!({
            "deleteAll": true,
            "namespace": type_name,
        });

        self.client
            .post(format!("{}/vectors/delete", self.url))
            .header("Api-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("pinecone clear failed: {e}")))?;

        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::Bool(b) => Value::Bool(*b),
        _ => Value::Null,
    }
}
