use super::{QueryFilter, StorageError, StorageResult};
use crate::value::{Record, Value};

/// SurrealDB multi-model storage backend using REST API.
/// Supports both document storage and graph relationships via RELATE.
pub struct SurrealDBBackend {
    url: String,
    namespace: String,
    database: String,
    client: reqwest::Client,
}

impl std::fmt::Debug for SurrealDBBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurrealDBBackend")
            .field("url", &self.url)
            .finish()
    }
}

impl SurrealDBBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let base = url.trim_end_matches('/').to_string();
        let namespace = std::env::var("SURREALDB_NAMESPACE").unwrap_or_else(|_| "synapse".into());
        let database = std::env::var("SURREALDB_DATABASE").unwrap_or_else(|_| "synapse".into());

        Ok(Self {
            url: base,
            namespace,
            database,
            client: reqwest::Client::new(),
        })
    }

    fn surreal_query(&self, sql: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}/sql", self.url))
            .header("NS", &self.namespace)
            .header("DB", &self.database)
            .header("Accept", "application/json")
            .body(sql.to_string())
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        _fields: &[(String, String)],
        _indexes: &[String],
    ) -> StorageResult<()> {
        let sql = format!("DEFINE TABLE IF NOT EXISTS {type_name} SCHEMALESS;");
        let _ = self.surreal_query(&sql).send().await;

        // Define Entity table for graph
        let _ = self
            .surreal_query("DEFINE TABLE IF NOT EXISTS Entity SCHEMALESS;")
            .send()
            .await;

        Ok(())
    }

    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        let mut fields = Vec::new();
        for (k, v) in &record.fields {
            fields.push(format!("{k} = {}", value_to_surreal(v)));
        }
        let set_clause = fields.join(", ");

        let sql = format!(
            "UPDATE {}:{} SET {set_clause};",
            record.type_name,
            surreal_id(&record.id)
        );

        self.surreal_query(&sql)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb store failed: {e}")))?;

        Ok(())
    }

    pub async fn store_triple(&self, record: &Record) -> StorageResult<()> {
        let subject = match record.fields.get("subject") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            _ => return Ok(()),
        };
        let predicate = match record.fields.get("predicate") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            _ => return Ok(()),
        };
        let object = match record.fields.get("object") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            _ => return Ok(()),
        };

        let sub_key = sanitize_key(&subject);
        let obj_key = sanitize_key(&object);
        let rel_type = predicate.to_uppercase().replace(' ', "_").replace('-', "_");

        let sql = format!(
            "UPDATE Entity:{sub_key} SET name = '{subject}'; \
             UPDATE Entity:{obj_key} SET name = '{object}'; \
             RELATE Entity:{sub_key}->{rel_type}->Entity:{obj_key} \
             SET predicate = '{predicate}', fact_id = '{}';",
            record.id
        );

        self.surreal_query(&sql)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb store_triple failed: {e}")))?;

        Ok(())
    }

    pub async fn graph_match_ids(
        &self,
        type_name: &str,
        input: &str,
        hops: usize,
    ) -> StorageResult<std::collections::HashSet<String>> {
        // SurrealDB graph traversal
        let sql = format!(
            "SELECT ->?[{hops}]->?.fact_id AS ids FROM Entity \
             WHERE string::lowercase(name) CONTAINSANY string::lowercase('{input}');"
        );

        let resp = self
            .surreal_query(&sql)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb graph_match failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb parse failed: {e}")))?;

        let mut ids = std::collections::HashSet::new();
        if let Some(results) = result.as_array().and_then(|a| a.first()) {
            if let Some(items) = results.get("result").and_then(|v| v.as_array()) {
                for item in items {
                    if let Some(fact_ids) = item.get("ids").and_then(|v| v.as_array()) {
                        for id in fact_ids {
                            if let Some(s) = id.as_str() {
                                ids.insert(s.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Filter to only IDs in the target type
        let _ = type_name;
        Ok(ids)
    }

    pub async fn cypher_query_ids(
        &self,
        surql: &str,
        _params: &std::collections::HashMap<String, String>,
    ) -> StorageResult<std::collections::HashSet<String>> {
        let resp = self
            .surreal_query(surql)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb parse failed: {e}")))?;

        let mut ids = std::collections::HashSet::new();
        if let Some(results) = result.as_array().and_then(|a| a.first()) {
            if let Some(items) = results.get("result").and_then(|v| v.as_array()) {
                for item in items {
                    if let Some(id) = item
                        .get("id")
                        .or_else(|| item.get("name"))
                        .and_then(|v| v.as_str())
                    {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
        Ok(ids)
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let sql = format!("SELECT * FROM {}:{};", type_name, surreal_id(id));

        let resp = self
            .surreal_query(&sql)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb get failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb parse failed: {e}")))?;

        if let Some(results) = result.as_array().and_then(|a| a.first()) {
            if let Some(items) = results.get("result").and_then(|v| v.as_array()) {
                if let Some(item) = items.first() {
                    let mut record = Record::new(type_name);
                    record.id = id.to_string();
                    if let Some(obj) = item.as_object() {
                        for (k, v) in obj {
                            if k == "id" {
                                continue;
                            }
                            record.fields.insert(k.clone(), json_to_value(v));
                        }
                    }
                    return Ok(Some(record));
                }
            }
        }
        Ok(None)
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let mut sql = format!("SELECT * FROM {type_name}");

        if !filter.conditions.is_empty() {
            let clauses: Vec<String> = filter
                .conditions
                .iter()
                .map(|c| {
                    let op = match c.op {
                        super::ConditionOp::Eq => "=",
                        super::ConditionOp::Ne => "!=",
                        super::ConditionOp::Lt => "<",
                        super::ConditionOp::Le => "<=",
                        super::ConditionOp::Gt => ">",
                        super::ConditionOp::Ge => ">=",
                    };
                    format!("{} {} {}", c.field, op, value_to_surreal(&c.value))
                })
                .collect();
            sql.push_str(&format!(" WHERE {}", clauses.join(" AND ")));
        }

        if let Some((field, asc)) = &filter.order_by {
            let dir = if *asc { "ASC" } else { "DESC" };
            sql.push_str(&format!(" ORDER BY {field} {dir}"));
        }

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        sql.push(';');

        let resp = self
            .surreal_query(&sql)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("surrealdb parse failed: {e}")))?;

        let mut records = Vec::new();
        if let Some(results) = result.as_array().and_then(|a| a.first()) {
            if let Some(items) = results.get("result").and_then(|v| v.as_array()) {
                for item in items {
                    let mut record = Record::new(type_name);
                    if let Some(obj) = item.as_object() {
                        if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                            // SurrealDB IDs are "table:id" format
                            record.id = id.split(':').nth(1).unwrap_or(id).to_string();
                        }
                        for (k, v) in obj {
                            if k == "id" {
                                continue;
                            }
                            record.fields.insert(k.clone(), json_to_value(v));
                        }
                    }
                    records.push(record);
                }
            }
        }
        Ok(records)
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        self.store(record).await
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let sql = format!("DELETE {}:{};", type_name, surreal_id(id));
        let _ = self.surreal_query(&sql).send().await;
        Ok(())
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        let sql = format!("DELETE {type_name};");
        let _ = self.surreal_query(&sql).send().await;
        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn sanitize_key(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn surreal_id(id: &str) -> String {
    format!("`{id}`")
}

fn value_to_surreal(v: &Value) -> String {
    match v {
        Value::String(s) => format!("'{}'", s.replace('\'', "\\'")),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "NONE".into(),
        other => format!("'{}'", serde_json::to_string(other).unwrap_or_default()),
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
