pub mod neo4j;
pub mod qdrant;
pub mod sqlite;

use std::sync::Arc;

use crate::llm::EmbeddingClient;
use crate::value::{Record, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(String),
    #[error("Qdrant error: {0}")]
    Qdrant(String),
    #[error("Neo4j error: {0}")]
    Neo4j(String),
    #[error("Not connected: {0}")]
    NotConnected(String),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Enum dispatch over storage backends — no dyn, no Box.
#[derive(Debug)]
pub enum StorageBackend {
    Sqlite(sqlite::SqliteBackend),
    Qdrant(qdrant::QdrantBackend),
    Neo4j(neo4j::Neo4jBackend),
}

/// Macro to dispatch method calls across all storage backend variants.
macro_rules! dispatch {
    ($self:expr, $method:ident ( $($arg:expr),* )) => {
        match $self {
            StorageBackend::Sqlite(s) => s.$method($($arg),*).await,
            StorageBackend::Qdrant(s) => s.$method($($arg),*).await,
            StorageBackend::Neo4j(s) => s.$method($($arg),*).await,
        }
    };
}

impl StorageBackend {
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        dispatch!(self, store(record))
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        dispatch!(self, get(type_name, id))
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        dispatch!(self, query(type_name, filter))
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        dispatch!(self, update(record))
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        dispatch!(self, delete(type_name, id))
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
    ) -> StorageResult<()> {
        dispatch!(self, ensure_table(type_name, fields))
    }
}

/// Filter for querying records — used by the query executor.
#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    pub conditions: Vec<Condition>,
    pub order_by: Option<(String, bool)>, // (field, ascending)
    pub limit: Option<usize>,
    pub graph_match: Option<GraphMatch>,
    pub cypher_query: Option<CypherQuery>,
    pub semantic_match: Option<SemanticMatch>,
    /// Regex filters applied after DB query: (field_name, regex)
    #[allow(clippy::type_complexity)]
    pub regex_filters: Vec<(String, regex::Regex)>,
    /// Raw SQL query to run instead of the normal table query
    pub raw_sql: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub field: String,
    pub op: ConditionOp,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum ConditionOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Graph traversal filter: find records connected within N hops of the input.
#[derive(Debug, Clone)]
pub struct GraphMatch {
    pub input: String,
    pub hops: usize,
}

/// Raw Cypher query to execute against the graph backend.
#[derive(Debug, Clone)]
pub struct CypherQuery {
    pub query: String,
    pub params: std::collections::HashMap<String, String>,
}

/// Semantic similarity search via vector embeddings.
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub input: String,
    pub threshold: f64,
}

/// Combined storage manager — holds all active backends.
#[derive(Debug)]
pub struct StorageManager {
    pub relational: Option<StorageBackend>,
    pub vector: Option<StorageBackend>,
    pub graph: Option<StorageBackend>,
    pub embedder: Option<Arc<EmbeddingClient>>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            relational: None,
            vector: None,
            graph: None,
            embedder: None,
        }
    }

    /// Store a record across all configured backends.
    /// SQLite gets all records. Qdrant gets records for embedding.
    /// Neo4j gets the node and, if the record has subject/predicate/object
    /// fields, also creates a relationship triple.
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        tracing::debug!(
            type_name = %record.type_name, id = %record.id,
            fields = ?record.fields.keys().collect::<Vec<_>>(),
            "storing record"
        );
        if let Some(ref r) = self.relational {
            r.store(record).await.map_err(|e| {
                tracing::error!(error = %e, type_name = %record.type_name, "SQLite store failed");
                e
            })?;
            tracing::debug!(type_name = %record.type_name, "stored to SQLite");
        }
        if let Some(ref v) = self.vector {
            v.store(record).await.map_err(|e| {
                tracing::error!(error = %e, type_name = %record.type_name, "Qdrant store failed");
                e
            })?;
            tracing::debug!(type_name = %record.type_name, "stored to Qdrant");
        }
        if let Some(StorageBackend::Neo4j(ref neo)) = self.graph {
            neo.store(record).await.map_err(|e| {
                tracing::error!(error = %e, type_name = %record.type_name, "Neo4j store failed");
                e
            })?;
            neo.store_triple(record).await.map_err(|e| {
                tracing::error!(error = %e, type_name = %record.type_name, "Neo4j store_triple failed");
                e
            })?;
            tracing::debug!(type_name = %record.type_name, "stored to Neo4j");
        }
        Ok(())
    }

    /// Fetch full records by IDs from graph or vector backends (fallback
    /// when the relational backend has no data for the queried type).
    async fn fetch_records_by_ids(
        &self,
        type_name: &str,
        ids: &std::collections::HashSet<String>,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        let mut results = Vec::new();

        // Try graph backend first (it stores richer record data)
        if let Some(ref g) = self.graph {
            let all = g.query(type_name, &QueryFilter::default()).await?;
            for r in all {
                if ids.contains(&r.id) {
                    results.push(r);
                }
            }
        }

        // If graph didn't have them, try vector backend
        if results.is_empty() {
            if let Some(ref v) = self.vector {
                let all = v.query(type_name, &QueryFilter::default()).await?;
                for r in all {
                    if ids.contains(&r.id) {
                        results.push(r);
                    }
                }
            }
        }

        // Apply remaining filter conditions in memory
        for c in &filter.conditions {
            results.retain(|r| {
                let field_val = r.fields.get(&c.field).unwrap_or(&Value::Null);
                match c.op {
                    ConditionOp::Eq => field_val == &c.value,
                    ConditionOp::Ne => field_val != &c.value,
                    _ => true,
                }
            });
        }

        // Apply ordering
        if let Some((ref field, asc)) = filter.order_by {
            results.sort_by(|a, b| {
                let va = a.fields.get(field);
                let vb = b.fields.get(field);
                let ord = match (va, vb) {
                    (Some(Value::Float(fa)), Some(Value::Float(fb))) => {
                        fa.partial_cmp(fb).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Some(Value::Int(ia)), Some(Value::Int(ib))) => ia.cmp(ib),
                    (Some(Value::String(sa)), Some(Value::String(sb))) => sa.cmp(sb),
                    _ => std::cmp::Ordering::Equal,
                };
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        tracing::debug!(
            count = results.len(),
            "fetched {} records from alternate backends",
            results.len()
        );
        Ok(results)
    }

    /// Query with multi-backend support.
    ///
    /// Strategy: gather candidate IDs from graph and semantic backends,
    /// then try to fetch matching records from the relational backend.
    /// If relational is empty/unavailable, fall back to fetching records
    /// directly from whichever backend has them (graph or vector).
    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        // If ordering by _score (virtual field), strip it from the filter
        // so backends don't choke, and apply it in memory after scoring.
        let score_order = match &filter.order_by {
            Some((field, asc)) if field == "_score" => Some(*asc),
            _ => None,
        };
        let filter = if score_order.is_some() {
            &QueryFilter {
                order_by: None,
                ..filter.clone()
            }
        } else {
            filter
        };

        let mut graph_ids: Option<std::collections::HashSet<String>> = None;

        if let Some(StorageBackend::Neo4j(ref neo)) = self.graph {
            if let Some(ref gm) = filter.graph_match {
                let ids = neo.graph_match_ids(type_name, &gm.input, gm.hops).await?;
                tracing::debug!(count = ids.len(), input = %gm.input, "graph_match returned IDs: {:?}", ids);
                graph_ids = Some(ids);
            }
            if let Some(ref cq) = filter.cypher_query {
                let ids = neo.cypher_query_ids(&cq.query, &cq.params).await?;
                match graph_ids {
                    Some(ref mut existing) => existing.retain(|id| ids.contains(id)),
                    None => graph_ids = Some(ids),
                }
            }
        }

        let mut semantic_ids: Option<std::collections::HashSet<String>> = None;
        let mut semantic_scores: std::collections::HashMap<String, f32> =
            std::collections::HashMap::new();
        if let Some(StorageBackend::Qdrant(ref qdrant)) = self.vector {
            if let Some(ref sm) = filter.semantic_match {
                if let Some(ref embedder) = self.embedder {
                    match embedder.embed(&sm.input).await {
                        Ok(vector) => {
                            let limit = filter.limit.unwrap_or(20);
                            match qdrant
                                .search_by_vector(type_name, vector, limit, sm.threshold)
                                .await
                            {
                                Ok(scored) => {
                                    tracing::debug!(count = scored.len(), input = %sm.input, "semantic_match returned {} results", scored.len());
                                    if !scored.is_empty() {
                                        let mut ids = std::collections::HashSet::new();
                                        for (id, score) in scored {
                                            ids.insert(id.clone());
                                            semantic_scores.insert(id, score);
                                        }
                                        semantic_ids = Some(ids);
                                    } else {
                                        tracing::debug!("semantic_match returned 0 results, treating as unconstrained");
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "semantic search failed, proceeding without");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "embedding generation for query failed");
                        }
                    }
                }
            }
        }

        // Union graph and semantic candidate IDs — either backend can
        // contribute relevant results rather than requiring both to agree.
        let candidate_ids: Option<std::collections::HashSet<String>> =
            match (&graph_ids, &semantic_ids) {
                (Some(g), Some(s)) => {
                    let union: std::collections::HashSet<String> = g.union(s).cloned().collect();
                    tracing::debug!(
                        graph = g.len(),
                        semantic = s.len(),
                        union = union.len(),
                        "unioned candidate IDs"
                    );
                    Some(union)
                }
                (Some(g), None) => Some(g.clone()),
                (None, Some(s)) => Some(s.clone()),
                (None, None) => None,
            };

        tracing::debug!(type_name = %type_name, conditions = ?filter.conditions, limit = ?filter.limit, "query filter");

        let mut results = if let Some(ref raw_sql) = filter.raw_sql {
            if let Some(StorageBackend::Sqlite(ref sqlite)) = self.relational {
                sqlite.raw_sql(raw_sql)?
            } else {
                vec![]
            }
        } else if let Some(ref r) = self.relational {
            r.query(type_name, filter).await?
        } else {
            vec![]
        };

        tracing::debug!(
            count = results.len(),
            "relational query returned {} records",
            results.len()
        );

        if let Some(ref ids) = candidate_ids {
            if results.is_empty() {
                // Relational backend has no data — fall back to fetching records
                // from graph or vector backends using candidate IDs.
                tracing::debug!(
                    candidates = ids.len(),
                    "relational empty, fetching from alternate backends"
                );
                results = self.fetch_records_by_ids(type_name, ids, filter).await?;
            } else {
                let before = results.len();
                results.retain(|r| ids.contains(&r.id));
                tracing::debug!(
                    before = before,
                    after = results.len(),
                    "filtered relational results by candidate IDs"
                );
            }
        }

        for (field, re) in &filter.regex_filters {
            results.retain(|r| {
                r.fields
                    .get(field)
                    .and_then(|v| v.as_str())
                    .map(|s| re.is_match(s))
                    .unwrap_or(false)
            });
        }

        // Attach _score from semantic search to each record
        if !semantic_scores.is_empty() {
            for r in &mut results {
                if let Some(&score) = semantic_scores.get(&r.id) {
                    r.set("_score", Value::Float(score as f64));
                }
            }
        }

        // Apply _score ordering in memory (since it's a virtual field)
        if let Some(asc) = score_order {
            results.sort_by(|a, b| {
                let sa = a
                    .fields
                    .get("_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let sb = b
                    .fields
                    .get("_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let ord = sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal);
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }

        Ok(results)
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        if let Some(ref r) = self.relational {
            return r.get(type_name, id).await;
        }
        Ok(None)
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        if let Some(ref r) = self.relational {
            r.delete(type_name, id).await?;
        }
        if let Some(StorageBackend::Qdrant(ref qdrant)) = self.vector {
            qdrant.delete(type_name, id).await?;
        }
        if let Some(ref g) = self.graph {
            g.delete(type_name, id).await?;
        }
        Ok(())
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
    ) -> StorageResult<()> {
        if let Some(ref r) = self.relational {
            r.ensure_table(type_name, fields).await?;
        }
        if let Some(ref v) = self.vector {
            v.ensure_table(type_name, fields).await?;
        }
        Ok(())
    }

    /// Clear all records from all backends for the given memory types.
    pub async fn clear(&self, memory_names: &[&str]) -> StorageResult<serde_json::Value> {
        let mut report = serde_json::Map::new();

        if let Some(StorageBackend::Sqlite(ref sqlite)) = self.relational {
            let mut cleared = Vec::new();
            for name in memory_names {
                match sqlite.clear(name).await {
                    Ok(()) => cleared.push(name.to_string()),
                    Err(e) => {
                        tracing::warn!(table = name, error = %e, "failed to clear SQLite table")
                    }
                }
            }
            report.insert("sqlite".into(), serde_json::json!(cleared));
        }

        if let Some(StorageBackend::Qdrant(ref qdrant)) = self.vector {
            let mut cleared = Vec::new();
            for name in memory_names {
                match qdrant.clear(name).await {
                    Ok(()) => cleared.push(name.to_string()),
                    Err(e) => {
                        tracing::warn!(collection = name, error = %e, "failed to clear Qdrant collection")
                    }
                }
            }
            report.insert("qdrant".into(), serde_json::json!(cleared));
        }

        if let Some(StorageBackend::Neo4j(ref neo)) = self.graph {
            let mut cleared = Vec::new();
            for name in memory_names {
                match neo.clear(name).await {
                    Ok(()) => cleared.push(name.to_string()),
                    Err(e) => {
                        tracing::warn!(label = name, error = %e, "failed to clear Neo4j label")
                    }
                }
            }
            // Also clear Entity nodes
            if let Err(e) = neo.clear("Entity").await {
                tracing::warn!(error = %e, "failed to clear Neo4j Entity nodes");
            } else {
                cleared.push("Entity".into());
            }
            report.insert("neo4j".into(), serde_json::json!(cleared));
        }

        Ok(serde_json::Value::Object(report))
    }

    /// Execute raw SQL against the relational backend.
    pub fn raw_sql(&self, sql: &str) -> StorageResult<Vec<crate::value::Record>> {
        if let Some(StorageBackend::Sqlite(ref sqlite)) = self.relational {
            sqlite.raw_sql(sql)
        } else {
            Err(StorageError::NotConnected(
                "no relational backend for raw SQL".into(),
            ))
        }
    }

    /// Inspect all backends: list tables/collections and record counts.
    pub async fn inspect(&self, memory_names: &[&str]) -> serde_json::Value {
        let mut result = serde_json::json!({});

        // SQLite
        if let Some(StorageBackend::Sqlite(ref sqlite)) = self.relational {
            let mut tables = serde_json::Map::new();
            for name in memory_names {
                match sqlite.raw_sql(&format!("SELECT * FROM {name}")) {
                    Ok(records) => {
                        let rows: Vec<serde_json::Value> = records
                            .into_iter()
                            .map(|r| serde_json::Value::from(Value::Record(r)))
                            .collect();
                        tables.insert(
                            name.to_string(),
                            serde_json::json!({
                                "count": rows.len(),
                                "records": rows,
                            }),
                        );
                    }
                    Err(e) => {
                        tables.insert(
                            name.to_string(),
                            serde_json::json!({
                                "error": e.to_string(),
                            }),
                        );
                    }
                }
            }
            result["sqlite"] = serde_json::Value::Object(tables);
        } else {
            result["sqlite"] = serde_json::json!("not configured");
        }

        // Qdrant
        if let Some(StorageBackend::Qdrant(ref qdrant)) = self.vector {
            let mut collections = serde_json::Map::new();
            for name in memory_names {
                let filter = QueryFilter::default();
                match qdrant.query(name, &filter).await {
                    Ok(records) => {
                        let rows: Vec<serde_json::Value> = records
                            .into_iter()
                            .map(|r| serde_json::Value::from(Value::Record(r)))
                            .collect();
                        collections.insert(
                            name.to_string(),
                            serde_json::json!({
                                "count": rows.len(),
                                "records": rows,
                            }),
                        );
                    }
                    Err(e) => {
                        collections.insert(
                            name.to_string(),
                            serde_json::json!({
                                "error": e.to_string(),
                            }),
                        );
                    }
                }
            }
            result["qdrant"] = serde_json::Value::Object(collections);
        } else {
            result["qdrant"] = serde_json::json!("not configured");
        }

        // Neo4j
        if let Some(StorageBackend::Neo4j(ref neo)) = self.graph {
            let mut nodes = serde_json::Map::new();
            for name in memory_names {
                let filter = QueryFilter::default();
                match neo.query(name, &filter).await {
                    Ok(records) => {
                        let rows: Vec<serde_json::Value> = records
                            .into_iter()
                            .map(|r| serde_json::Value::from(Value::Record(r)))
                            .collect();
                        nodes.insert(
                            name.to_string(),
                            serde_json::json!({
                                "count": rows.len(),
                                "records": rows,
                            }),
                        );
                    }
                    Err(e) => {
                        nodes.insert(
                            name.to_string(),
                            serde_json::json!({
                                "error": e.to_string(),
                            }),
                        );
                    }
                }
            }
            // Also grab Entity nodes
            let filter = QueryFilter::default();
            if let Ok(records) = neo.query("Entity", &filter).await {
                let rows: Vec<serde_json::Value> = records
                    .into_iter()
                    .map(|r| serde_json::Value::from(Value::Record(r)))
                    .collect();
                nodes.insert(
                    "Entity".to_string(),
                    serde_json::json!({
                        "count": rows.len(),
                        "records": rows,
                    }),
                );
            }
            result["neo4j"] = serde_json::Value::Object(nodes);
        } else {
            result["neo4j"] = serde_json::json!("not configured");
        }

        result
    }
}
