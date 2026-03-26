use std::sync::Arc;

use super::{QueryFilter, StorageError, StorageResult};
use crate::llm::EmbeddingClient;
use crate::value::{Record, Value};

/// Weaviate vector storage backend using REST API.
pub struct WeaviateBackend {
    url: String,
    client: reqwest::Client,
    embedder: Option<Arc<EmbeddingClient>>,
}

impl std::fmt::Debug for WeaviateBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeaviateBackend")
            .field("url", &self.url)
            .finish()
    }
}

impl WeaviateBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let client = reqwest::Client::new();
        let base = url.trim_end_matches('/').to_string();

        // Verify connection
        client
            .get(format!("{base}/v1/.well-known/ready"))
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate connection failed: {e}")))?;

        Ok(Self {
            url: base,
            client,
            embedder: None,
        })
    }

    pub fn set_embedder(&mut self, embedder: Arc<EmbeddingClient>) {
        self.embedder = Some(embedder);
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
        _indexes: &[String],
    ) -> StorageResult<()> {
        let properties: Vec<serde_json::Value> = fields
            .iter()
            .map(|(name, ty)| {
                let weaviate_type = match ty.as_str() {
                    "int" => "int",
                    "float" => "number",
                    "bool" => "boolean",
                    _ => "text",
                };
                serde_json::json!({
                    "name": name,
                    "dataType": [weaviate_type],
                })
            })
            .collect();

        let body = serde_json::json!({
            "class": type_name,
            "properties": properties,
            "vectorizer": "none",
        });

        let resp = self
            .client
            .post(format!("{}/v1/schema", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate schema creation failed: {e}")))?;

        if resp.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            // Class likely already exists
            return Ok(());
        }

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
            self.store_without_vector(record).await
        }
    }

    async fn store_without_vector(&self, record: &Record) -> StorageResult<()> {
        let mut properties = serde_json::Map::new();
        for (k, v) in &record.fields {
            match v {
                Value::String(s) => {
                    properties.insert(k.clone(), s.clone().into());
                }
                Value::Int(n) => {
                    properties.insert(k.clone(), (*n).into());
                }
                Value::Float(f) => {
                    properties.insert(k.clone(), (*f).into());
                }
                Value::Bool(b) => {
                    properties.insert(k.clone(), (*b).into());
                }
                _ => {}
            }
        }
        properties.insert("_id".into(), record.id.clone().into());

        let body = serde_json::json!({
            "class": record.type_name,
            "id": uuid_from_string(&record.id),
            "properties": properties,
        });

        self.client
            .post(format!("{}/v1/objects", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate store failed: {e}")))?;

        Ok(())
    }

    pub async fn store_with_vector(&self, record: &Record, vector: Vec<f32>) -> StorageResult<()> {
        let mut properties = serde_json::Map::new();
        for (k, v) in &record.fields {
            match v {
                Value::String(s) => {
                    properties.insert(k.clone(), s.clone().into());
                }
                Value::Int(n) => {
                    properties.insert(k.clone(), (*n).into());
                }
                Value::Float(f) => {
                    properties.insert(k.clone(), (*f).into());
                }
                Value::Bool(b) => {
                    properties.insert(k.clone(), (*b).into());
                }
                _ => {}
            }
        }
        properties.insert("_id".into(), record.id.clone().into());

        let body = serde_json::json!({
            "class": record.type_name,
            "id": uuid_from_string(&record.id),
            "properties": properties,
            "vector": vector,
        });

        self.client
            .post(format!("{}/v1/objects", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate store failed: {e}")))?;

        Ok(())
    }

    pub async fn search_by_vector(
        &self,
        type_name: &str,
        vector: Vec<f32>,
        limit: usize,
        threshold: f64,
    ) -> StorageResult<Vec<(String, f32)>> {
        let graphql = serde_json::json!({
            "query": format!(
                r#"{{
                    Get {{
                        {type_name}(
                            nearVector: {{ vector: {vector:?}, certainty: {threshold} }}
                            limit: {limit}
                        ) {{
                            _id
                            _additional {{ id certainty }}
                        }}
                    }}
                }}"#,
            ),
        });

        let resp = self
            .client
            .post(format!("{}/v1/graphql", self.url))
            .json(&graphql)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate search failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate parse failed: {e}")))?;

        let mut scored = Vec::new();
        if let Some(items) = result
            .pointer(&format!("/data/Get/{type_name}"))
            .and_then(|v| v.as_array())
        {
            for item in items {
                let id = item.get("_id").and_then(|v| v.as_str()).unwrap_or_default();
                let score = item
                    .pointer("/_additional/certainty")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                scored.push((id.to_string(), score));
            }
        }
        Ok(scored)
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let weaviate_id = uuid_from_string(id);
        let resp = self
            .client
            .get(format!("{}/v1/objects/{type_name}/{weaviate_id}", self.url))
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate get failed: {e}")))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate parse failed: {e}")))?;

        let mut record = Record::new(type_name);
        record.id = id.to_string();
        if let Some(props) = result.get("properties").and_then(|p| p.as_object()) {
            for (k, v) in props {
                if k == "_id" {
                    continue;
                }
                record.fields.insert(k.clone(), json_to_value(v));
            }
        }
        Ok(Some(record))
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let limit = filter.limit.unwrap_or(100);
        let graphql = serde_json::json!({
            "query": format!(
                r#"{{ Get {{ {type_name}(limit: {limit}) {{ _id _additional {{ id }} }} }} }}"#,
            ),
        });

        let resp = self
            .client
            .post(format!("{}/v1/graphql", self.url))
            .json(&graphql)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate parse failed: {e}")))?;

        let mut records = Vec::new();
        if let Some(items) = result
            .pointer(&format!("/data/Get/{type_name}"))
            .and_then(|v| v.as_array())
        {
            for item in items {
                let mut record = Record::new(type_name);
                if let Some(id) = item.get("_id").and_then(|v| v.as_str()) {
                    record.id = id.to_string();
                }
                if let Some(obj) = item.as_object() {
                    for (k, v) in obj {
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
        let weaviate_id = uuid_from_string(id);
        self.client
            .delete(format!("{}/v1/objects/{type_name}/{weaviate_id}", self.url))
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate delete failed: {e}")))?;
        Ok(())
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        let body = serde_json::json!({
            "match": {
                "class": type_name,
            },
        });

        self.client
            .delete(format!("{}/v1/batch/objects", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Qdrant(format!("weaviate clear failed: {e}")))?;

        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn uuid_from_string(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    let hash = hasher.finish();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (hash >> 32) as u32,
        (hash >> 16) as u16,
        (hash & 0xFFFF) as u16,
        ((hash >> 48) & 0xFFFF) as u16,
        hash & 0xFFFF_FFFF_FFFF
    )
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
