pub mod neo4j;
pub mod qdrant;
pub mod sqlite;

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

/// Combined storage manager — holds all active backends.
#[derive(Debug)]
pub struct StorageManager {
    pub relational: Option<StorageBackend>,
    pub vector: Option<StorageBackend>,
    pub graph: Option<StorageBackend>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            relational: None,
            vector: None,
            graph: None,
        }
    }

    /// Store a record across all configured backends.
    /// SQLite gets all records. Qdrant gets records for embedding.
    /// Neo4j gets the node and, if the record has subject/predicate/object
    /// fields, also creates a relationship triple.
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        if let Some(ref r) = self.relational {
            r.store(record).await?;
        }
        if let Some(ref v) = self.vector {
            v.store(record).await?;
        }
        if let Some(StorageBackend::Neo4j(ref neo)) = self.graph {
            neo.store(record).await?;
            neo.store_triple(record).await?;
        }
        Ok(())
    }

    /// Query with multi-backend support.
    /// 1. If graph conditions exist and a graph backend is configured,
    ///    run the graph query first to get candidate IDs.
    /// 2. Query relational with those IDs as an additional filter.
    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let mut graph_ids: Option<std::collections::HashSet<String>> = None;

        if let Some(StorageBackend::Neo4j(ref neo)) = self.graph {
            if let Some(ref gm) = filter.graph_match {
                let ids = neo.graph_match_ids(type_name, &gm.input, gm.hops).await?;
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

        let mut results = if let Some(ref r) = self.relational {
            r.query(type_name, filter).await?
        } else {
            vec![]
        };

        if let Some(ref ids) = graph_ids {
            results.retain(|r| ids.contains(&r.id));
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
        Ok(())
    }
}
