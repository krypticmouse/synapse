pub mod arangodb;
pub mod chromadb;
pub mod memgraph;
pub mod neo4j;
pub mod pinecone;
pub mod qdrant;
pub mod sqlite;
pub mod surrealdb;
pub mod weaviate;

use std::collections::HashMap;
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

/// Enum dispatch over relational storage backends.
#[derive(Debug)]
pub enum StorageBackend {
    Sqlite(sqlite::SqliteBackend),
}

impl StorageBackend {
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        match self {
            StorageBackend::Sqlite(s) => s.store(record).await,
        }
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        match self {
            StorageBackend::Sqlite(s) => s.get(type_name, id).await,
        }
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        match self {
            StorageBackend::Sqlite(s) => s.query(type_name, filter).await,
        }
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        match self {
            StorageBackend::Sqlite(s) => s.update(record).await,
        }
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        match self {
            StorageBackend::Sqlite(s) => s.delete(type_name, id).await,
        }
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
        indexes: &[String],
    ) -> StorageResult<()> {
        match self {
            StorageBackend::Sqlite(s) => s.ensure_table(type_name, fields, indexes).await,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// VECTOR BACKEND
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub enum VectorBackendKind {
    Qdrant(qdrant::QdrantBackend),
    Pinecone(pinecone::PineconeBackend),
    Weaviate(weaviate::WeaviateBackend),
    ChromaDB(chromadb::ChromaDBBackend),
}

impl VectorBackendKind {
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        match self {
            VectorBackendKind::Qdrant(s) => s.store(record).await,
            VectorBackendKind::Pinecone(s) => s.store(record).await,
            VectorBackendKind::Weaviate(s) => s.store(record).await,
            VectorBackendKind::ChromaDB(s) => s.store(record).await,
        }
    }

    pub async fn search_by_vector(
        &self,
        type_name: &str,
        vector: Vec<f32>,
        limit: usize,
        threshold: f64,
    ) -> StorageResult<Vec<(String, f32)>> {
        match self {
            VectorBackendKind::Qdrant(s) => {
                s.search_by_vector(type_name, vector, limit, threshold)
                    .await
            }
            VectorBackendKind::Pinecone(s) => {
                s.search_by_vector(type_name, vector, limit, threshold)
                    .await
            }
            VectorBackendKind::Weaviate(s) => {
                s.search_by_vector(type_name, vector, limit, threshold)
                    .await
            }
            VectorBackendKind::ChromaDB(s) => {
                s.search_by_vector(type_name, vector, limit, threshold)
                    .await
            }
        }
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        match self {
            VectorBackendKind::Qdrant(s) => s.get(type_name, id).await,
            VectorBackendKind::Pinecone(s) => s.get(type_name, id).await,
            VectorBackendKind::Weaviate(s) => s.get(type_name, id).await,
            VectorBackendKind::ChromaDB(s) => s.get(type_name, id).await,
        }
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        match self {
            VectorBackendKind::Qdrant(s) => s.query(type_name, filter).await,
            VectorBackendKind::Pinecone(s) => s.query(type_name, filter).await,
            VectorBackendKind::Weaviate(s) => s.query(type_name, filter).await,
            VectorBackendKind::ChromaDB(s) => s.query(type_name, filter).await,
        }
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        match self {
            VectorBackendKind::Qdrant(s) => s.delete(type_name, id).await,
            VectorBackendKind::Pinecone(s) => s.delete(type_name, id).await,
            VectorBackendKind::Weaviate(s) => s.delete(type_name, id).await,
            VectorBackendKind::ChromaDB(s) => s.delete(type_name, id).await,
        }
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        match self {
            VectorBackendKind::Qdrant(s) => s.clear(type_name).await,
            VectorBackendKind::Pinecone(s) => s.clear(type_name).await,
            VectorBackendKind::Weaviate(s) => s.clear(type_name).await,
            VectorBackendKind::ChromaDB(s) => s.clear(type_name).await,
        }
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
        indexes: &[String],
    ) -> StorageResult<()> {
        match self {
            VectorBackendKind::Qdrant(s) => s.ensure_table(type_name, fields, indexes).await,
            VectorBackendKind::Pinecone(s) => s.ensure_table(type_name, fields, indexes).await,
            VectorBackendKind::Weaviate(s) => s.ensure_table(type_name, fields, indexes).await,
            VectorBackendKind::ChromaDB(s) => s.ensure_table(type_name, fields, indexes).await,
        }
    }

    pub fn set_embedder(&mut self, embedder: Arc<EmbeddingClient>) {
        match self {
            VectorBackendKind::Qdrant(s) => s.set_embedder(embedder),
            VectorBackendKind::Pinecone(s) => s.set_embedder(embedder),
            VectorBackendKind::Weaviate(s) => s.set_embedder(embedder),
            VectorBackendKind::ChromaDB(s) => s.set_embedder(embedder),
        }
    }

    pub fn url(&self) -> &str {
        match self {
            VectorBackendKind::Qdrant(s) => s.url(),
            VectorBackendKind::Pinecone(s) => s.url(),
            VectorBackendKind::Weaviate(s) => s.url(),
            VectorBackendKind::ChromaDB(s) => s.url(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// GRAPH BACKEND
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub enum GraphBackendKind {
    Neo4j(neo4j::Neo4jBackend),
    Memgraph(memgraph::MemgraphBackend),
    ArangoDB(arangodb::ArangoDBBackend),
    SurrealDB(surrealdb::SurrealDBBackend),
}

impl GraphBackendKind {
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        match self {
            GraphBackendKind::Neo4j(s) => s.store(record).await,
            GraphBackendKind::Memgraph(s) => s.store(record).await,
            GraphBackendKind::ArangoDB(s) => s.store(record).await,
            GraphBackendKind::SurrealDB(s) => s.store(record).await,
        }
    }

    pub async fn store_triple(&self, record: &Record) -> StorageResult<()> {
        match self {
            GraphBackendKind::Neo4j(s) => s.store_triple(record).await,
            GraphBackendKind::Memgraph(s) => s.store_triple(record).await,
            GraphBackendKind::ArangoDB(s) => s.store_triple(record).await,
            GraphBackendKind::SurrealDB(s) => s.store_triple(record).await,
        }
    }

    pub async fn graph_match_ids(
        &self,
        type_name: &str,
        input: &str,
        hops: usize,
    ) -> StorageResult<std::collections::HashSet<String>> {
        match self {
            GraphBackendKind::Neo4j(s) => s.graph_match_ids(type_name, input, hops).await,
            GraphBackendKind::Memgraph(s) => s.graph_match_ids(type_name, input, hops).await,
            GraphBackendKind::ArangoDB(s) => s.graph_match_ids(type_name, input, hops).await,
            GraphBackendKind::SurrealDB(s) => s.graph_match_ids(type_name, input, hops).await,
        }
    }

    pub async fn cypher_query_ids(
        &self,
        query: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> StorageResult<std::collections::HashSet<String>> {
        match self {
            GraphBackendKind::Neo4j(s) => s.cypher_query_ids(query, params).await,
            GraphBackendKind::Memgraph(s) => s.cypher_query_ids(query, params).await,
            GraphBackendKind::ArangoDB(s) => s.cypher_query_ids(query, params).await,
            GraphBackendKind::SurrealDB(s) => s.cypher_query_ids(query, params).await,
        }
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        match self {
            GraphBackendKind::Neo4j(s) => s.get(type_name, id).await,
            GraphBackendKind::Memgraph(s) => s.get(type_name, id).await,
            GraphBackendKind::ArangoDB(s) => s.get(type_name, id).await,
            GraphBackendKind::SurrealDB(s) => s.get(type_name, id).await,
        }
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        match self {
            GraphBackendKind::Neo4j(s) => s.query(type_name, filter).await,
            GraphBackendKind::Memgraph(s) => s.query(type_name, filter).await,
            GraphBackendKind::ArangoDB(s) => s.query(type_name, filter).await,
            GraphBackendKind::SurrealDB(s) => s.query(type_name, filter).await,
        }
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        match self {
            GraphBackendKind::Neo4j(s) => s.delete(type_name, id).await,
            GraphBackendKind::Memgraph(s) => s.delete(type_name, id).await,
            GraphBackendKind::ArangoDB(s) => s.delete(type_name, id).await,
            GraphBackendKind::SurrealDB(s) => s.delete(type_name, id).await,
        }
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        match self {
            GraphBackendKind::Neo4j(s) => s.clear(type_name).await,
            GraphBackendKind::Memgraph(s) => s.clear(type_name).await,
            GraphBackendKind::ArangoDB(s) => s.clear(type_name).await,
            GraphBackendKind::SurrealDB(s) => s.clear(type_name).await,
        }
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
        indexes: &[String],
    ) -> StorageResult<()> {
        match self {
            GraphBackendKind::Neo4j(s) => s.ensure_table(type_name, fields, indexes).await,
            GraphBackendKind::Memgraph(s) => s.ensure_table(type_name, fields, indexes).await,
            GraphBackendKind::ArangoDB(s) => s.ensure_table(type_name, fields, indexes).await,
            GraphBackendKind::SurrealDB(s) => s.ensure_table(type_name, fields, indexes).await,
        }
    }

    pub fn url(&self) -> &str {
        match self {
            GraphBackendKind::Neo4j(s) => s.url(),
            GraphBackendKind::Memgraph(s) => s.url(),
            GraphBackendKind::ArangoDB(s) => s.url(),
            GraphBackendKind::SurrealDB(s) => s.url(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// QUERY FILTER
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    pub conditions: Vec<Condition>,
    pub or_conditions: Vec<Condition>,
    pub order_by: Option<(String, bool)>,
    pub limit: Option<usize>,
    pub graph_match: Option<GraphMatch>,
    pub cypher_query: Option<CypherQuery>,
    pub semantic_match: Option<SemanticMatch>,
    #[allow(clippy::type_complexity)]
    pub regex_filters: Vec<(String, regex::Regex)>,
    pub raw_sql: Option<String>,
    /// Target vector backend name for semantic_match (None = use first/default)
    pub vector_backend: Option<String>,
    /// Target graph backend name for graph_match/cypher (None = use first/default)
    pub graph_backend: Option<String>,
    /// Score aliases: alias_name -> scoring function type ("semantic" or "graph")
    pub score_aliases: HashMap<String, String>,
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

#[derive(Debug, Clone)]
pub struct GraphMatch {
    pub input: String,
    pub hops: usize,
}

#[derive(Debug, Clone)]
pub struct CypherQuery {
    pub query: String,
    pub params: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub input: String,
    pub threshold: f64,
}

// ═══════════════════════════════════════════════════════════════
// STORAGE MANAGER (multi-backend)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct StorageManager {
    pub relational: Option<StorageBackend>,
    pub vectors: HashMap<String, VectorBackendKind>,
    pub graphs: HashMap<String, GraphBackendKind>,
    pub embedder: Option<Arc<EmbeddingClient>>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            relational: None,
            vectors: HashMap::new(),
            graphs: HashMap::new(),
            embedder: None,
        }
    }

    /// Get the first (or named) vector backend
    pub fn vector(&self, name: Option<&str>) -> Option<&VectorBackendKind> {
        match name {
            Some(n) => self.vectors.get(n),
            None => self
                .vectors
                .get("default")
                .or_else(|| self.vectors.values().next()),
        }
    }

    /// Get the first (or named) graph backend
    pub fn graph(&self, name: Option<&str>) -> Option<&GraphBackendKind> {
        match name {
            Some(n) => self.graphs.get(n),
            None => self
                .graphs
                .get("default")
                .or_else(|| self.graphs.values().next()),
        }
    }

    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        tracing::debug!(
            type_name = %record.type_name, id = %record.id,
            fields = ?record.fields.keys().collect::<Vec<_>>(),
            "storing record"
        );
        if let Some(ref r) = self.relational {
            r.store(record).await.map_err(|e| {
                tracing::error!(error = %e, type_name = %record.type_name, "relational store failed");
                e
            })?;
        }
        // Store to ALL vector backends
        for (name, v) in &self.vectors {
            if let Err(e) = v.store(record).await {
                tracing::error!(error = %e, backend = %name, type_name = %record.type_name, "vector store failed");
            }
        }
        // Store to ALL graph backends (node + triples)
        for (name, g) in &self.graphs {
            if let Err(e) = g.store(record).await {
                tracing::error!(error = %e, backend = %name, type_name = %record.type_name, "graph store failed");
            }
            if let Err(e) = g.store_triple(record).await {
                tracing::error!(error = %e, backend = %name, type_name = %record.type_name, "graph store_triple failed");
            }
        }
        Ok(())
    }

    async fn fetch_records_by_ids(
        &self,
        type_name: &str,
        ids: &std::collections::HashSet<String>,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        let mut results = Vec::new();

        for g in self.graphs.values() {
            let all = g.query(type_name, &QueryFilter::default()).await?;
            for r in all {
                let id_match = ids.contains(&r.id);
                let name_match = r
                    .fields
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|n| ids.contains(n))
                    .unwrap_or(false);
                if id_match || name_match {
                    results.push(r);
                }
            }
            if !results.is_empty() {
                break;
            }
        }

        if results.is_empty() {
            for v in self.vectors.values() {
                let all = v.query(type_name, &QueryFilter::default()).await?;
                for r in all {
                    let id_match = ids.contains(&r.id);
                    let name_match = r
                        .fields
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|n| ids.contains(n))
                        .unwrap_or(false);
                    if id_match || name_match {
                        results.push(r);
                    }
                }
                if !results.is_empty() {
                    break;
                }
            }
        }

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

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn fallback_query_all(
        &self,
        type_name: &str,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        let mut results = Vec::new();

        for g in self.graphs.values() {
            results = g.query(type_name, &QueryFilter::default()).await?;
            if !results.is_empty() {
                break;
            }
        }
        if results.is_empty() {
            for v in self.vectors.values() {
                results = v.query(type_name, &QueryFilter::default()).await?;
                if !results.is_empty() {
                    break;
                }
            }
        }

        fn matches_cond(field_val: &Value, op: &ConditionOp, target: &Value) -> bool {
            match op {
                ConditionOp::Eq => field_val == target,
                ConditionOp::Ne => field_val != target,
                ConditionOp::Lt | ConditionOp::Le | ConditionOp::Gt | ConditionOp::Ge => true,
            }
        }

        for c in &filter.conditions {
            results.retain(|r| {
                let fv = r.fields.get(&c.field).unwrap_or(&Value::Null);
                matches_cond(fv, &c.op, &c.value)
            });
        }

        if !filter.or_conditions.is_empty() {
            results.retain(|r| {
                filter.or_conditions.iter().any(|c| {
                    let fv = r.fields.get(&c.field).unwrap_or(&Value::Null);
                    matches_cond(fv, &c.op, &c.value)
                })
            });
        }

        if let Some((ref field, asc)) = filter.order_by {
            results.sort_by(|a, b| {
                let va = a.fields.get(field);
                let vb = b.fields.get(field);
                let ord = match (va, vb) {
                    (Some(Value::String(sa)), Some(Value::String(sb))) => sa.cmp(sb),
                    (Some(Value::Int(ia)), Some(Value::Int(ib))) => ia.cmp(ib),
                    (Some(Value::Float(fa)), Some(Value::Float(fb))) => {
                        fa.partial_cmp(fb).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    _ => std::cmp::Ordering::Equal,
                };
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Query with multi-backend support.
    /// Returns (results, score_maps) where score_maps maps alias names to per-record scores.
    pub async fn query_with_scores(
        &self,
        type_name: &str,
        filter: &QueryFilter,
    ) -> StorageResult<(Vec<Record>, HashMap<String, HashMap<String, f64>>)> {
        let mut alias_scores: HashMap<String, HashMap<String, f64>> = HashMap::new();

        let has_candidate_backends = filter.graph_match.is_some()
            || filter.semantic_match.is_some()
            || filter.cypher_query.is_some();

        let deferred_limit = if has_candidate_backends {
            filter.limit
        } else {
            None
        };

        // Strip virtual _score or alias-based ordering from backend filters
        let is_virtual_order = filter.order_by.as_ref().map_or(false, |(field, _)| {
            field == "_score" || filter.score_aliases.contains_key(field)
        });

        let filter = if is_virtual_order || has_candidate_backends {
            &QueryFilter {
                order_by: if is_virtual_order {
                    None
                } else {
                    filter.order_by.clone()
                },
                limit: if has_candidate_backends {
                    None
                } else {
                    filter.limit
                },
                ..filter.clone()
            }
        } else {
            filter
        };

        let mut graph_ids: Option<std::collections::HashSet<String>> = None;

        // Use targeted or first graph backend
        let graph_backend = self.graph(filter.graph_backend.as_deref());
        if let Some(gb) = graph_backend {
            if let Some(ref gm) = filter.graph_match {
                let ids = gb.graph_match_ids(type_name, &gm.input, gm.hops).await?;
                // If there's a graph alias, record a score of 1.0 for matched IDs
                for (alias, kind) in &filter.score_aliases {
                    if kind == "graph" {
                        let scores = alias_scores.entry(alias.clone()).or_default();
                        for id in &ids {
                            scores.insert(id.clone(), 1.0);
                        }
                    }
                }
                graph_ids = Some(ids);
            }
            if let Some(ref cq) = filter.cypher_query {
                let ids = gb.cypher_query_ids(&cq.query, &cq.params).await?;
                match graph_ids {
                    Some(ref mut existing) => existing.retain(|id| ids.contains(id)),
                    None => graph_ids = Some(ids),
                }
            }
        }

        let mut semantic_ids: Option<std::collections::HashSet<String>> = None;
        let mut semantic_scores: std::collections::HashMap<String, f32> =
            std::collections::HashMap::new();

        let vector_backend = self.vector(filter.vector_backend.as_deref());
        if let Some(vb) = vector_backend {
            if let Some(ref sm) = filter.semantic_match {
                if let Some(ref embedder) = self.embedder {
                    match embedder.embed(&sm.input).await {
                        Ok(vector) => {
                            let limit = filter.limit.unwrap_or(20);
                            match vb
                                .search_by_vector(type_name, vector, limit, sm.threshold)
                                .await
                            {
                                Ok(scored) => {
                                    if !scored.is_empty() {
                                        let mut ids = std::collections::HashSet::new();
                                        for (id, score) in scored {
                                            ids.insert(id.clone());
                                            semantic_scores.insert(id, score);
                                        }
                                        semantic_ids = Some(ids);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "semantic search failed");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "embedding generation failed");
                        }
                    }
                }
            }
        }

        // Record semantic scores under their aliases
        for (alias, kind) in &filter.score_aliases {
            if kind == "semantic" {
                let scores = alias_scores.entry(alias.clone()).or_default();
                for (id, score) in &semantic_scores {
                    scores.insert(id.clone(), *score as f64);
                }
            }
        }

        let candidate_ids: Option<std::collections::HashSet<String>> =
            match (&graph_ids, &semantic_ids) {
                (Some(g), Some(s)) => Some(g.union(s).cloned().collect()),
                (Some(g), None) => Some(g.clone()),
                (None, Some(s)) => Some(s.clone()),
                (None, None) => None,
            };

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

        if let Some(ref ids) = candidate_ids {
            if results.is_empty() {
                results = self.fetch_records_by_ids(type_name, ids, filter).await?;
            } else {
                results.retain(|r| ids.contains(&r.id));
            }
        } else if results.is_empty() && (!self.graphs.is_empty() || !self.vectors.is_empty()) {
            results = self.fallback_query_all(type_name, filter).await?;
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

        // Attach _score (backward compat: from semantic search)
        if !semantic_scores.is_empty() {
            for r in &mut results {
                if let Some(&score) = semantic_scores.get(&r.id) {
                    r.set("_score", Value::Float(score as f64));
                }
            }
        }

        if let Some(limit) = deferred_limit {
            results.truncate(limit);
        }

        Ok((results, alias_scores))
    }

    /// Backward-compatible query method (no alias scores)
    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let (results, _) = self.query_with_scores(type_name, filter).await?;
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
        for v in self.vectors.values() {
            let _ = v.delete(type_name, id).await;
        }
        for g in self.graphs.values() {
            let _ = g.delete(type_name, id).await;
        }
        Ok(())
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
        indexes: &[String],
    ) -> StorageResult<()> {
        if let Some(ref r) = self.relational {
            r.ensure_table(type_name, fields, indexes).await?;
        }
        for v in self.vectors.values() {
            v.ensure_table(type_name, fields, indexes).await?;
        }
        for g in self.graphs.values() {
            g.ensure_table(type_name, fields, indexes).await?;
        }
        Ok(())
    }

    pub async fn clear(&self, memory_names: &[&str]) -> StorageResult<serde_json::Value> {
        let mut report = serde_json::Map::new();

        if let Some(StorageBackend::Sqlite(ref sqlite)) = self.relational {
            let mut cleared = Vec::new();
            for name in memory_names {
                if sqlite.clear(name).await.is_ok() {
                    cleared.push(name.to_string());
                }
            }
            report.insert("sqlite".into(), serde_json::json!(cleared));
        }

        for (backend_name, v) in &self.vectors {
            let mut cleared = Vec::new();
            for name in memory_names {
                if v.clear(name).await.is_ok() {
                    cleared.push(name.to_string());
                }
            }
            report.insert(format!("vector_{backend_name}"), serde_json::json!(cleared));
        }

        for (backend_name, g) in &self.graphs {
            let mut cleared = Vec::new();
            for name in memory_names {
                if g.clear(name).await.is_ok() {
                    cleared.push(name.to_string());
                }
            }
            let _ = g.clear("Entity").await;
            report.insert(format!("graph_{backend_name}"), serde_json::json!(cleared));
        }

        Ok(serde_json::Value::Object(report))
    }

    pub fn raw_sql(&self, sql: &str) -> StorageResult<Vec<crate::value::Record>> {
        if let Some(StorageBackend::Sqlite(ref sqlite)) = self.relational {
            sqlite.raw_sql(sql)
        } else {
            Err(StorageError::NotConnected(
                "no relational backend for raw SQL".into(),
            ))
        }
    }

    pub async fn inspect(&self, memory_names: &[&str]) -> serde_json::Value {
        let mut result = serde_json::json!({});

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
                            serde_json::json!({ "count": rows.len(), "records": rows }),
                        );
                    }
                    Err(e) => {
                        tables.insert(
                            name.to_string(),
                            serde_json::json!({ "error": e.to_string() }),
                        );
                    }
                }
            }
            result["sqlite"] = serde_json::Value::Object(tables);
        } else {
            result["sqlite"] = serde_json::json!("not configured");
        }

        for (backend_name, vb) in &self.vectors {
            let mut collections = serde_json::Map::new();
            for name in memory_names {
                let filter = QueryFilter::default();
                match vb.query(name, &filter).await {
                    Ok(records) => {
                        let rows: Vec<serde_json::Value> = records
                            .into_iter()
                            .map(|r| serde_json::Value::from(Value::Record(r)))
                            .collect();
                        collections.insert(
                            name.to_string(),
                            serde_json::json!({ "count": rows.len(), "records": rows }),
                        );
                    }
                    Err(e) => {
                        collections.insert(
                            name.to_string(),
                            serde_json::json!({ "error": e.to_string() }),
                        );
                    }
                }
            }
            result[format!("vector_{backend_name}")] = serde_json::Value::Object(collections);
        }

        for (backend_name, gb) in &self.graphs {
            let mut nodes = serde_json::Map::new();
            for name in memory_names {
                let filter = QueryFilter::default();
                match gb.query(name, &filter).await {
                    Ok(records) => {
                        let rows: Vec<serde_json::Value> = records
                            .into_iter()
                            .map(|r| serde_json::Value::from(Value::Record(r)))
                            .collect();
                        nodes.insert(
                            name.to_string(),
                            serde_json::json!({ "count": rows.len(), "records": rows }),
                        );
                    }
                    Err(e) => {
                        nodes.insert(
                            name.to_string(),
                            serde_json::json!({ "error": e.to_string() }),
                        );
                    }
                }
            }
            result[format!("graph_{backend_name}")] = serde_json::Value::Object(nodes);
        }

        result
    }
}
