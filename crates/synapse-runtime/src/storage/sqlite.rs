use rusqlite::{params_from_iter, Connection};
use std::path::Path;
use std::sync::Mutex;

use super::{ConditionOp, QueryFilter, StorageError, StorageResult};
use crate::value::{Record, Value};

#[derive(Debug)]
pub struct SqliteBackend {
    conn: Mutex<Connection>,
}

impl SqliteBackend {
    pub fn open(path: &str) -> StorageResult<Self> {
        let p = Path::new(path);
        if p != Path::new(":memory:") {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    StorageError::Sqlite(format!("failed to create data directory: {e}"))
                })?;
            }
        }
        let conn = Connection::open(path).map_err(|e| StorageError::Sqlite(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory() -> StorageResult<Self> {
        Self::open(":memory:")
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        fields: &[(String, String)],
        indexes: &[String],
    ) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        let columns: Vec<String> = fields
            .iter()
            .map(|(name, ty)| format!("{name} {}", sql_type(ty)))
            .collect();
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {type_name} (
                _id TEXT PRIMARY KEY,
                {}
            )",
            columns.join(", ")
        );
        conn.execute(&sql, [])
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        for field in indexes {
            let idx_name = format!("idx_{type_name}_{field}");
            let idx_sql = format!("CREATE INDEX IF NOT EXISTS {idx_name} ON {type_name} ({field})");
            conn.execute(&idx_sql, [])
                .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();

        let mut field_names: Vec<&str> = vec!["_id"];
        let mut placeholders: Vec<String> = vec!["?".into()];
        let mut values: Vec<rusqlite::types::Value> = vec![record.id.clone().into()];

        for (name, value) in &record.fields {
            field_names.push(name);
            placeholders.push("?".into());
            values.push(value_to_sqlite(value));
        }

        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            record.type_name,
            field_names.join(", "),
            placeholders.join(", ")
        );

        conn.execute(&sql, params_from_iter(values))
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(())
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT * FROM {type_name} WHERE _id = ?");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let columns: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let result = stmt
            .query_row([id], |row| {
                let mut record = Record::new(type_name);
                for (i, col) in columns.iter().enumerate() {
                    if col == "_id" {
                        record.id = row.get::<_, String>(i).unwrap_or_default();
                    } else {
                        let val = sqlite_to_value(row, i);
                        record.fields.insert(col.clone(), val);
                    }
                }
                Ok(record)
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => return Ok(None),
                other => Err(StorageError::Sqlite(other.to_string())),
            });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(Ok(none)) => Ok(none),
            Err(Err(e)) => Err(e),
        }
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = format!("SELECT * FROM {type_name}");
        let mut bind_values: Vec<rusqlite::types::Value> = vec![];

        let has_and = !filter.conditions.is_empty();
        let has_or = !filter.or_conditions.is_empty();

        if has_and || has_or {
            let mut where_parts: Vec<String> = Vec::new();

            if has_and {
                let clauses: Vec<String> = filter
                    .conditions
                    .iter()
                    .map(|c| condition_to_sql(c, &mut bind_values))
                    .collect();
                where_parts.push(clauses.join(" AND "));
            }

            if has_or {
                let or_clauses: Vec<String> = filter
                    .or_conditions
                    .iter()
                    .map(|c| condition_to_sql(c, &mut bind_values))
                    .collect();
                where_parts.push(format!("({})", or_clauses.join(" OR ")));
            }

            sql.push_str(&format!(" WHERE {}", where_parts.join(" AND ")));
        }

        if let Some((field, asc)) = &filter.order_by {
            let dir = if *asc { "ASC" } else { "DESC" };
            sql.push_str(&format!(" ORDER BY {field} {dir}"));
        }

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let columns: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt
            .query_map(params_from_iter(bind_values), |row| {
                let mut record = Record::new(type_name);
                for (i, col) in columns.iter().enumerate() {
                    if col == "_id" {
                        record.id = row.get::<_, String>(i).unwrap_or_default();
                    } else {
                        record.fields.insert(col.clone(), sqlite_to_value(row, i));
                    }
                }
                Ok(record)
            })
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let mut results = vec![];
        for row in rows {
            results.push(row.map_err(|e| StorageError::Sqlite(e.to_string()))?);
        }
        Ok(results)
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        // Use INSERT OR REPLACE (upsert)
        self.store(record).await
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("DELETE FROM {type_name} WHERE _id = ?");
        conn.execute(&sql, [id])
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(())
    }

    /// Delete all rows from the given table.
    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("DELETE FROM {type_name}");
        conn.execute(&sql, [])
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(())
    }

    /// Execute a raw SQL query and return results as Records.
    pub fn raw_sql(&self, sql: &str) -> StorageResult<Vec<Record>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let columns: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt
            .query_map([], |row| {
                let mut record = Record::new("_sql_result");
                for (i, col) in columns.iter().enumerate() {
                    if col == "_id" {
                        record.id = row.get::<_, String>(i).unwrap_or_default();
                    } else {
                        record.fields.insert(col.clone(), sqlite_to_value(row, i));
                    }
                }
                Ok(record)
            })
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let mut results = vec![];
        for row in rows {
            results.push(row.map_err(|e| StorageError::Sqlite(e.to_string()))?);
        }
        Ok(results)
    }
}

fn condition_to_sql(c: &super::Condition, bind_values: &mut Vec<rusqlite::types::Value>) -> String {
    if matches!(c.value, Value::Null) {
        return match c.op {
            ConditionOp::Eq => format!("{} IS NULL", c.field),
            ConditionOp::Ne => format!("{} IS NOT NULL", c.field),
            _ => {
                bind_values.push(value_to_sqlite(&c.value));
                format!("{} = ?", c.field)
            }
        };
    }
    let op = match c.op {
        ConditionOp::Eq => "=",
        ConditionOp::Ne => "!=",
        ConditionOp::Lt => "<",
        ConditionOp::Le => "<=",
        ConditionOp::Gt => ">",
        ConditionOp::Ge => ">=",
    };
    bind_values.push(value_to_sqlite(&c.value));
    format!("{} {} ?", c.field, op)
}

fn sql_type(synapse_type: &str) -> &str {
    match synapse_type {
        "string" => "TEXT",
        "int" => "INTEGER",
        "float" | "bounded_float" => "REAL",
        "bool" => "INTEGER",
        "timestamp" => "TEXT",
        _ => "TEXT",
    }
}

fn value_to_sqlite(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Int(n) => rusqlite::types::Value::Integer(*n),
        Value::Float(f) => rusqlite::types::Value::Real(*f),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Timestamp(t) => rusqlite::types::Value::Text(t.to_rfc3339()),
        Value::Array(a) => {
            rusqlite::types::Value::Text(serde_json::to_string(a).unwrap_or_default())
        }
        Value::Record(r) => {
            rusqlite::types::Value::Text(serde_json::to_string(r).unwrap_or_default())
        }
    }
}

fn sqlite_to_value(row: &rusqlite::Row, idx: usize) -> Value {
    // Try types in order of specificity
    if let Ok(v) = row.get::<_, i64>(idx) {
        return Value::Int(v);
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return Value::Float(v);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return Value::String(v);
    }
    Value::Null
}
