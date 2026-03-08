use super::{QueryFilter, StorageError, StorageResult};
use crate::value::Record;

/// Neo4j graph storage backend for knowledge graph operations.
pub struct Neo4jBackend {
    url: String,
    graph: Option<neo4rs::Graph>,
}

impl std::fmt::Debug for Neo4jBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neo4jBackend").field("url", &self.url).finish()
    }
}

impl Neo4jBackend {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let graph = neo4rs::Graph::new(url, "", "")
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        Ok(Self {
            url: url.to_string(),
            graph: Some(graph),
        })
    }

    pub async fn ensure_table(
        &self,
        _type_name: &str,
        _fields: &[(String, String)],
    ) -> StorageResult<()> {
        // Neo4j doesn't require table creation, nodes are schema-free
        Ok(())
    }

    pub async fn store(&self, record: &Record) -> StorageResult<()> {
        let graph = self.graph.as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        // Store as a node with label = type_name
        let mut props = String::from("{_id: $id");
        for key in record.fields.keys() {
            props.push_str(&format!(", {key}: ${key}"));
        }
        props.push('}');

        let query_str = format!(
            "MERGE (n:{} {{_id: $id}}) SET n += {props}",
            record.type_name
        );

        let mut query = neo4rs::query(&query_str).param("id", record.id.clone());
        for (key, value) in &record.fields {
            let str_val = match value {
                crate::value::Value::String(s) => s.clone(),
                crate::value::Value::Int(n) => n.to_string(),
                crate::value::Value::Float(f) => f.to_string(),
                crate::value::Value::Bool(b) => b.to_string(),
                other => serde_json::to_string(other).unwrap_or_default(),
            };
            query = query.param(key.as_str(), str_val);
        }

        graph
            .run(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        Ok(())
    }

    pub async fn get(&self, type_name: &str, id: &str) -> StorageResult<Option<Record>> {
        let graph = self.graph.as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let query = neo4rs::query(&format!(
            "MATCH (n:{type_name} {{_id: $id}}) RETURN n"
        ))
        .param("id", id.to_string());

        let mut result = graph
            .execute(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?
        {
            let node: neo4rs::Node = row
                .get("n")
                .map_err(|e| StorageError::Neo4j(e.to_string()))?;

            let mut record = Record::new(type_name);
            record.id = id.to_string();

            for key in node.keys() {
                if key == "_id" {
                    continue;
                }
                if let Ok(val) = node.get::<String>(key) {
                    record.fields.insert(key.to_string(), crate::value::Value::String(val));
                }
            }

            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    pub async fn query(
        &self,
        type_name: &str,
        filter: &QueryFilter,
    ) -> StorageResult<Vec<Record>> {
        let graph = self.graph.as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let mut cypher = format!("MATCH (n:{type_name})");

        if !filter.conditions.is_empty() {
            let clauses: Vec<String> = filter
                .conditions
                .iter()
                .map(|c| {
                    let op = match c.op {
                        super::ConditionOp::Eq => "=",
                        super::ConditionOp::Ne => "<>",
                        super::ConditionOp::Lt => "<",
                        super::ConditionOp::Le => "<=",
                        super::ConditionOp::Gt => ">",
                        super::ConditionOp::Ge => ">=",
                    };
                    format!("n.{} {} '{}'", c.field, op, value_to_cypher_string(&c.value))
                })
                .collect();
            cypher.push_str(&format!(" WHERE {}", clauses.join(" AND ")));
        }

        cypher.push_str(" RETURN n");

        if let Some((field, asc)) = &filter.order_by {
            let dir = if *asc { "ASC" } else { "DESC" };
            cypher.push_str(&format!(" ORDER BY n.{field} {dir}"));
        }

        if let Some(limit) = filter.limit {
            cypher.push_str(&format!(" LIMIT {limit}"));
        }

        let query = neo4rs::query(&cypher);
        let mut result = graph
            .execute(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        let mut records = vec![];
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?
        {
            let node: neo4rs::Node = row
                .get("n")
                .map_err(|e| StorageError::Neo4j(e.to_string()))?;

            let mut record = Record::new(type_name);
            for key in node.keys() {
                if key == "_id" {
                    if let Ok(id) = node.get::<String>(key) {
                        record.id = id;
                    }
                } else if let Ok(val) = node.get::<String>(key) {
                    record.fields.insert(key.to_string(), crate::value::Value::String(val));
                }
            }
            records.push(record);
        }

        Ok(records)
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        self.store(record).await
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let graph = self.graph.as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let query = neo4rs::query(&format!(
            "MATCH (n:{type_name} {{_id: $id}}) DETACH DELETE n"
        ))
        .param("id", id.to_string());

        graph
            .run(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn value_to_cypher_string(value: &crate::value::Value) -> String {
    match value {
        crate::value::Value::String(s) => s.clone(),
        crate::value::Value::Int(n) => n.to_string(),
        crate::value::Value::Float(f) => f.to_string(),
        crate::value::Value::Bool(b) => b.to_string(),
        crate::value::Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}
