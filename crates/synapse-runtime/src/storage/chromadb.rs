use std::sync::Arc;

use super::{QueryFilter, StorageError, StorageResult};
use crate::llm::EmbeddingClient;
use crate::value::{Record, Value};

/// ChromaDB vector storage backend using REST API.
pub struct ChromaDBBackend {
    url: String,
    client: reqwest::Client,
    embedder: Option<Arc<EmbeddingClient>>,
    /// Cached collection IDs: type_name -> collection_id
    collections: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl std::fmt::Debug for ChromaDBBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChromaDBBackend")
            .field("url", &self.url)
            .finish()
    }
}

impl ChromaDBBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let base = url.trim_end_matches('/').to_string();
        Ok(Self {
            url: base,
            client: reqwest::Client::new(),
            embedder: None,
            collections: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }

    pub fn set_embedder(&mut self, embedder: Arc<EmbeddingClient>) {
        self.embedder = Some(embedder);
    }

    async fn get_or_create_collection(&self, name: &str) -> StorageResult<String> {
        {
            let cache = self.collections.lock().unwrap();
            if let Some(id) = cache.get(name) {
                return Ok(id.clone());
            }
        }

        let body = serde_json::json!({
            "name": name,
            "get_or_create": true,
        });

        let resp = self
            .client
            .post(format!("{}/api/v1/collections", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb create collection failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb parse failed: {e}")))?;

        let id = result
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(name)
            .to_string();

        self.collections
            .lock()
            .unwrap()
            .insert(name.to_string(), id.clone());
        Ok(id)
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        _fields: &[(String, String)],
        _indexes: &[String],
    ) -> StorageResult<()> {
        self.get_or_create_collection(type_name).await?;
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
        let collection_id = self.get_or_create_collection(&record.type_name).await?;

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

        let document = record
            .fields
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let body = serde_json::json!({
            "ids": [record.id],
            "embeddings": [vector.iter().map(|v| *v as f64).collect::<Vec<f64>>()],
            "metadatas": [metadata],
            "documents": [document],
        });

        self.client
            .post(format!(
                "{}/api/v1/collections/{collection_id}/upsert",
                self.url
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb upsert failed: {e}")))?;

        Ok(())
    }

    pub async fn search_by_vector(
        &self,
        type_name: &str,
        vector: Vec<f32>,
        limit: usize,
        threshold: f64,
    ) -> StorageResult<Vec<(String, f32)>> {
        let collection_id = self.get_or_create_collection(type_name).await?;

        let body = serde_json::json!({
            "query_embeddings": [vector.iter().map(|v| *v as f64).collect::<Vec<f64>>()],
            "n_results": limit,
            "include": ["metadatas", "distances"],
        });

        let resp = self
            .client
            .post(format!(
                "{}/api/v1/collections/{collection_id}/query",
                self.url
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb parse failed: {e}")))?;

        let mut scored = Vec::new();
        if let (Some(ids), Some(distances)) = (
            result.get("ids").and_then(|v| v.get(0)).and_then(|v| v.as_array()),
            result
                .get("distances")
                .and_then(|v| v.get(0))
                .and_then(|v| v.as_array()),
        ) {
            for (id_val, dist_val) in ids.iter().zip(distances.iter()) {
                let id = id_val.as_str().unwrap_or_default();
                let distance = dist_val.as_f64().unwrap_or(1.0);
                // ChromaDB returns distances (lower = better), convert to similarity score
                let score = (1.0 - distance).max(0.0) as f32;
                if score as f64 >= threshold {
                    scored.push((id.to_string(), score));
                }
            }
        }
        Ok(scored)
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let collection_id = self.get_or_create_collection(type_name).await?;

        let body = serde_json::json!({
            "ids": [id],
            "include": ["metadatas", "documents"],
        });

        let resp = self
            .client
            .post(format!(
                "{}/api/v1/collections/{collection_id}/get",
                self.url
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb get failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb parse failed: {e}")))?;

        if let Some(ids) = result.get("ids").and_then(|v| v.as_array()) {
            if !ids.is_empty() {
                let mut record = Record::new(type_name);
                record.id = id.to_string();
                if let Some(meta) = result
                    .get("metadatas")
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_object())
                {
                    for (k, v) in meta {
                        if k.starts_with('_') {
                            continue;
                        }
                        record.fields.insert(k.clone(), json_to_value(v));
                    }
                }
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    pub async fn query(
        &self,
        type_name: &str,
        _filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        let collection_id = self.get_or_create_collection(type_name).await?;

        let body = serde_json::json!({
            "include": ["metadatas", "documents"],
        });

        let resp = self
            .client
            .post(format!(
                "{}/api/v1/collections/{collection_id}/get",
                self.url
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb parse failed: {e}")))?;

        let mut records = Vec::new();
        if let (Some(ids), Some(metadatas)) = (
            result.get("ids").and_then(|v| v.as_array()),
            result.get("metadatas").and_then(|v| v.as_array()),
        ) {
            for (id_val, meta_val) in ids.iter().zip(metadatas.iter()) {
                let mut record = Record::new(type_name);
                record.id = id_val.as_str().unwrap_or_default().to_string();
                if let Some(meta) = meta_val.as_object() {
                    for (k, v) in meta {
                        if k.starts_with('_') {
                            continue;
                        }
                        record.fields.insert(k.clone(), json_to_value(v));
                    }
                }
                records.push(record);
            }
        }
        Ok(records)
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        self.store(record).await
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let collection_id = self.get_or_create_collection(type_name).await?;

        let body = serde_json::json!({
            "ids": [id],
        });

        self.client
            .post(format!(
                "{}/api/v1/collections/{collection_id}/delete",
                self.url
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("chromadb delete failed: {e}")))?;

        Ok(())
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        // Delete and recreate the collection
        if let Ok(collection_id) = self.get_or_create_collection(type_name).await {
            let _ = self
                .client
                .delete(format!(
                    "{}/api/v1/collections/{collection_id}",
                    self.url
                ))
                .send()
                .await;
            self.collections.lock().unwrap().remove(type_name);
            self.get_or_create_collection(type_name).await?;
        }
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
