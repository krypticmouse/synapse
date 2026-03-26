use super::{QueryFilter, StorageError, StorageResult};
use crate::value::{Record, Value};

/// ArangoDB graph storage backend using REST API with AQL graph queries.
pub struct ArangoDBBackend {
    url: String,
    database: String,
    client: reqwest::Client,
}

impl std::fmt::Debug for ArangoDBBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArangoDBBackend")
            .field("url", &self.url)
            .field("database", &self.database)
            .finish()
    }
}

impl ArangoDBBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let base = url.trim_end_matches('/').to_string();
        let database = std::env::var("ARANGODB_DATABASE").unwrap_or_else(|_| "synapse".into());

        let client = reqwest::Client::new();

        // Ensure database exists
        let _ = client
            .post(format!("{base}/_api/database"))
            .json(&serde_json::json!({ "name": database }))
            .send()
            .await;

        Ok(Self {
            url: base,
            database,
            client,
        })
    }

    fn db_url(&self) -> String {
        format!("{}/_db/{}", self.url, self.database)
    }

    pub async fn ensure_table(
        &self,
        type_name: &str,
        _fields: &[(String, String)],
        _indexes: &[String],
    ) -> StorageResult<()> {
        let body = serde_json::json!({ "name": type_name });
        let _ = self
            .client
            .post(format!("{}/_api/collection", self.db_url()))
            .json(&body)
            .send()
            .await;

        // Also create edge collection for relationships
        let edge_name = format!("{type_name}_edges");
        let _ = self
            .client
            .post(format!("{}/_api/collection", self.db_url()))
            .json(&serde_json::json!({ "name": edge_name, "type": 3 }))
            .send()
            .await;

        // Create Entity vertex collection
        let _ = self
            .client
            .post(format!("{}/_api/collection", self.db_url()))
            .json(&serde_json::json!({ "name": "Entity" }))
            .send()
            .await;

        Ok(())
    }

    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        let mut doc = serde_json::Map::new();
        doc.insert("_key".into(), record.id.clone().into());
        doc.insert("_id_field".into(), record.id.clone().into());
        for (k, v) in &record.fields {
            doc.insert(k.clone(), value_to_json(v));
        }

        self.client
            .post(format!(
                "{}/_api/document/{}",
                self.db_url(),
                record.type_name
            ))
            .query(&[("overwriteMode", "replace")])
            .json(&doc)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb store failed: {e}")))?;

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

        // Upsert subject entity
        let sub_key = sanitize_key(&subject);
        let _ = self
            .client
            .post(format!("{}/_api/document/Entity", self.db_url()))
            .query(&[("overwriteMode", "ignore")])
            .json(&serde_json::json!({
                "_key": sub_key,
                "name": subject,
            }))
            .send()
            .await;

        // Upsert object entity
        let obj_key = sanitize_key(&object);
        let _ = self
            .client
            .post(format!("{}/_api/document/Entity", self.db_url()))
            .query(&[("overwriteMode", "ignore")])
            .json(&serde_json::json!({
                "_key": obj_key,
                "name": object,
            }))
            .send()
            .await;

        // Create edge
        let edge_collection = format!("{}_edges", record.type_name);
        let _ = self
            .client
            .post(format!("{}/_api/document/{edge_collection}", self.db_url()))
            .json(&serde_json::json!({
                "_from": format!("Entity/{sub_key}"),
                "_to": format!("Entity/{obj_key}"),
                "predicate": predicate,
                "fact_id": record.id,
            }))
            .send()
            .await;

        Ok(())
    }

    pub async fn graph_match_ids(
        &self,
        type_name: &str,
        input: &str,
        hops: usize,
    ) -> StorageResult<std::collections::HashSet<String>> {
        let aql = format!(
            r#"FOR entity IN Entity
                FILTER CONTAINS(LOWER($input), LOWER(entity.name))
                FOR v, e, p IN 1..{hops} ANY entity {type_name}_edges
                    FILTER IS_SAME_COLLECTION('{type_name}', v)
                    RETURN DISTINCT v._id_field"#
        );

        let body = serde_json::json!({
            "query": aql,
            "bindVars": { "input": input },
        });

        let resp = self
            .client
            .post(format!("{}/_api/cursor", self.db_url()))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb graph_match failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb parse failed: {e}")))?;

        let mut ids = std::collections::HashSet::new();
        if let Some(items) = result.get("result").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(id) = item.as_str() {
                    ids.insert(id.to_string());
                }
            }
        }
        Ok(ids)
    }

    pub async fn cypher_query_ids(
        &self,
        aql: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> StorageResult<std::collections::HashSet<String>> {
        let body = serde_json::json!({
            "query": aql,
            "bindVars": params,
        });

        let resp = self
            .client
            .post(format!("{}/_api/cursor", self.db_url()))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb parse failed: {e}")))?;

        let mut ids = std::collections::HashSet::new();
        if let Some(items) = result.get("result").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(s) = item.as_str() {
                    ids.insert(s.to_string());
                } else if let Some(obj) = item.as_object() {
                    if let Some(id) = obj
                        .get("_id_field")
                        .or_else(|| obj.get("name"))
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
        let resp = self
            .client
            .get(format!(
                "{}/_api/document/{type_name}/{id}",
                self.db_url()
            ))
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb get failed: {e}")))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb parse failed: {e}")))?;

        let mut record = Record::new(type_name);
        record.id = id.to_string();
        if let Some(obj) = result.as_object() {
            for (k, v) in obj {
                if k.starts_with('_') {
                    continue;
                }
                record.fields.insert(k.clone(), json_to_value(v));
            }
        }
        Ok(Some(record))
    }

    pub async fn query(
        &self,
        type_name: &str,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        let mut aql = format!("FOR doc IN {type_name}");

        if !filter.conditions.is_empty() {
            let clauses: Vec<String> = filter
                .conditions
                .iter()
                .map(|c| {
                    let op = match c.op {
                        super::ConditionOp::Eq => "==",
                        super::ConditionOp::Ne => "!=",
                        super::ConditionOp::Lt => "<",
                        super::ConditionOp::Le => "<=",
                        super::ConditionOp::Gt => ">",
                        super::ConditionOp::Ge => ">=",
                    };
                    format!("doc.{} {} '{}'", c.field, op, value_to_aql_string(&c.value))
                })
                .collect();
            aql.push_str(&format!(" FILTER {}", clauses.join(" AND ")));
        }

        if let Some((field, asc)) = &filter.order_by {
            let dir = if *asc { "ASC" } else { "DESC" };
            aql.push_str(&format!(" SORT doc.{field} {dir}"));
        }

        if let Some(limit) = filter.limit {
            aql.push_str(&format!(" LIMIT {limit}"));
        }

        aql.push_str(" RETURN doc");

        let body = serde_json::json!({ "query": aql });

        let resp = self
            .client
            .post(format!("{}/_api/cursor", self.db_url()))
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb query failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::Neo4j(format!("arangodb parse failed: {e}")))?;

        let mut records = Vec::new();
        if let Some(items) = result.get("result").and_then(|v| v.as_array()) {
            for item in items {
                let mut record = Record::new(type_name);
                if let Some(obj) = item.as_object() {
                    if let Some(id) = obj.get("_id_field").and_then(|v| v.as_str()) {
                        record.id = id.to_string();
                    } else if let Some(key) = obj.get("_key").and_then(|v| v.as_str()) {
                        record.id = key.to_string();
                    }
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
        let _ = self
            .client
            .delete(format!(
                "{}/_api/document/{type_name}/{id}",
                self.db_url()
            ))
            .send()
            .await;
        Ok(())
    }

    pub async fn clear(&self, type_name: &str) -> StorageResult<()> {
        let _ = self
            .client
            .put(format!("{}/_api/collection/{type_name}/truncate", self.db_url()))
            .send()
            .await;
        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn sanitize_key(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Int(n) => serde_json::json!(n),
        Value::Float(f) => serde_json::json!(f),
        Value::Bool(b) => serde_json::json!(b),
        Value::Null => serde_json::Value::Null,
        other => serde_json::json!(format!("{:?}", other)),
    }
}

fn value_to_aql_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
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
