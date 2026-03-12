use std::collections::HashMap;
use std::sync::Arc;

use super::{QueryFilter, StorageError, StorageResult};
use crate::llm::EmbeddingClient;
use crate::value::{Record, Value};

/// Qdrant vector storage backend for semantic search.
pub struct QdrantBackend {
    url: String,
    client: Option<qdrant_client::Qdrant>,
    embedder: Option<Arc<EmbeddingClient>>,
}

impl std::fmt::Debug for QdrantBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QdrantBackend")
            .field("url", &self.url)
            .finish()
    }
}

impl QdrantBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let client = qdrant_client::Qdrant::from_url(url)
            .build()
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        Ok(Self {
            url: url.to_string(),
            client: Some(client),
            embedder: None,
        })
    }

    /// Wire an embedding client so `store()` can auto-embed records.
    pub fn set_embedder(&mut self, embedder: Arc<EmbeddingClient>) {
        self.embedder = Some(embedder);
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        _fields: &[(String, String)],
    ) -> StorageResult<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        let exists = client
            .collection_exists(type_name)
            .await
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        if !exists {
            use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder};
            client
                .create_collection(
                    CreateCollectionBuilder::new(type_name)
                        .vectors_config(VectorParamsBuilder::new(1536, Distance::Cosine)),
                )
                .await
                .map_err(|e| StorageError::Qdrant(e.to_string()))?;
        }

        Ok(())
    }

    /// Store a record by auto-embedding its `content` field.
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
            tracing::warn!(
                type_name = %record.type_name,
                id = %record.id,
                "no embedder configured, skipping vector store"
            );
            Ok(())
        }
    }

    /// Store a record with a pre-computed embedding vector.
    pub async fn store_with_vector(&self, record: &Record, vector: Vec<f32>) -> StorageResult<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        use qdrant_client::qdrant::{PointStruct, UpsertPointsBuilder};

        let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
        payload.insert("_type".to_string(), record.type_name.clone().into());
        payload.insert("_id".to_string(), record.id.clone().into());
        for (k, v) in &record.fields {
            match v {
                Value::String(s) => {
                    payload.insert(k.clone(), s.clone().into());
                }
                Value::Int(n) => {
                    payload.insert(k.clone(), (*n).into());
                }
                Value::Float(f) => {
                    payload.insert(k.clone(), (*f).into());
                }
                Value::Bool(b) => {
                    payload.insert(k.clone(), (*b).into());
                }
                _ => {}
            }
        }

        let point = PointStruct::new(record.id.clone(), vector, payload);
        tracing::info!(
            type_name = %record.type_name,
            id = %record.id,
            "upserting point to qdrant"
        );
        let resp = client
            .upsert_points(UpsertPointsBuilder::new(&record.type_name, vec![point]))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, type_name = %record.type_name, id = %record.id, "qdrant upsert FAILED");
                StorageError::Qdrant(e.to_string())
            })?;
        tracing::info!(
            type_name = %record.type_name,
            id = %record.id,
            status = ?resp.result.map(|r| r.status),
            "qdrant upsert response"
        );
        Ok(())
    }

    /// Search by vector similarity, returning (id, score) pairs.
    pub async fn search_by_vector(
        &self,
        type_name: &str,
        vector: Vec<f32>,
        limit: usize,
        threshold: f64,
    ) -> StorageResult<Vec<(String, f32)>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        use qdrant_client::qdrant::SearchPointsBuilder;

        let results = client
            .search_points(
                SearchPointsBuilder::new(type_name, vector, limit as u64)
                    .with_payload(true)
                    .score_threshold(threshold as f32),
            )
            .await
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        let scored: Vec<(String, f32)> = results
            .result
            .iter()
            .filter_map(|point| {
                let id = point
                    .payload
                    .get("_id")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))?;
                Some((id, point.score))
            })
            .collect();

        Ok(scored)
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        use qdrant_client::qdrant::{Condition, Filter, ScrollPointsBuilder};

        let filter = Filter::must([Condition::matches("_id", id.to_string())]);
        let scroll = ScrollPointsBuilder::new(type_name)
            .filter(filter)
            .limit(1)
            .with_payload(true);

        let result = client
            .scroll(scroll)
            .await
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        if let Some(point) = result.result.first() {
            let mut record = Record::new(type_name);
            if let Some(id_val) = point.payload.get("_id") {
                if let Some(s) = id_val.as_str() {
                    record.id = s.to_string();
                }
            }
            for (k, v) in &point.payload {
                if k.starts_with('_') {
                    continue;
                }
                if let Some(s) = v.as_str() {
                    record
                        .fields
                        .insert(k.clone(), Value::String(s.to_string()));
                } else if let Some(n) = v.as_integer() {
                    record.fields.insert(k.clone(), Value::Int(n));
                } else if let Some(f) = v.as_double() {
                    record.fields.insert(k.clone(), Value::Float(f));
                } else if let Some(b) = v.as_bool() {
                    record.fields.insert(k.clone(), Value::Bool(b));
                }
            }
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        use qdrant_client::qdrant::ScrollPointsBuilder;

        let limit = filter.limit.unwrap_or(100) as u32;
        let scroll = ScrollPointsBuilder::new(type_name)
            .limit(limit)
            .with_payload(true);

        let result = client
            .scroll(scroll)
            .await
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        let mut records = Vec::new();
        for point in &result.result {
            let mut record = Record::new(type_name);
            if let Some(id_val) = point.payload.get("_id") {
                if let Some(s) = id_val.as_str() {
                    record.id = s.to_string();
                }
            }
            for (k, v) in &point.payload {
                if k.starts_with('_') {
                    continue;
                }
                if let Some(s) = v.as_str() {
                    record
                        .fields
                        .insert(k.clone(), Value::String(s.to_string()));
                } else if let Some(n) = v.as_integer() {
                    record.fields.insert(k.clone(), Value::Int(n));
                } else if let Some(f) = v.as_double() {
                    record.fields.insert(k.clone(), Value::Float(f));
                } else if let Some(b) = v.as_bool() {
                    record.fields.insert(k.clone(), Value::Bool(b));
                }
            }
            records.push(record);
        }
        Ok(records)
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        self.store(record).await
    }

    /// Set the embedding dimension for collections (e.g. 1536 for text-embedding-3-small).
    pub fn embedding_dim(&self) -> usize {
        1536
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        use qdrant_client::qdrant::{DeletePointsBuilder, PointsIdsList};

        client
            .delete_points(DeletePointsBuilder::new(type_name).points(PointsIdsList {
                ids: vec![id.to_string().into()],
            }))
            .await
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        Ok(())
    }

    /// Delete all points inside a collection without dropping the collection itself.
    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("qdrant".into()))?;

        use qdrant_client::qdrant::{DeletePointsBuilder, Filter};

        // Empty filter matches all points
        client
            .delete_points(DeletePointsBuilder::new(type_name).points(Filter::default()))
            .await
            .map_err(|e| StorageError::Qdrant(e.to_string()))?;

        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}
