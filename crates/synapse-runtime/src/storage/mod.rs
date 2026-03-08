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

    pub async fn query(
        &self,
        type_name: &str,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        dispatch!(self, query(type_name, filter))
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        dispatch!(self, update(record))
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        dispatch!(self, delete(type_name, id))
    }

    pub async fn ensure_table(&self, type_name: &str, fields: &[(String, String)]) -> StorageResult<()> {
        dispatch!(self, ensure_table(type_name, fields))
    }
}

/// Filter for querying records — used by the query executor.
#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    pub conditions: Vec<Condition>,
    pub order_by: Option<(String, bool)>, // (field, ascending)
    pub limit: Option<usize>,
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

    /// Store a record across all configured backends
    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        if let Some(ref r) = self.relational {
            r.store(record).await?;
        }
        // Vector and graph backends can store embeddings/triplets
        // independently when configured
        Ok(())
    }

    pub async fn query(
        &self,
        type_name: &str,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        if let Some(ref r) = self.relational {
            return r.query(type_name, filter).await;
        }
        Ok(vec![])
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

    pub async fn ensure_table(&self, type_name: &str, fields: &[(String, String)]) -> StorageResult<()> {
        if let Some(ref r) = self.relational {
            r.ensure_table(type_name, fields).await?;
        }
        Ok(())
    }
}
