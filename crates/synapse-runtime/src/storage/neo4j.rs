use super::{QueryFilter, StorageError, StorageResult};
use crate::value::Record;

/// Neo4j graph storage backend for knowledge graph operations.
pub struct Neo4jBackend {
    url: String,
    graph: Option<neo4rs::Graph>,
}

impl std::fmt::Debug for Neo4jBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neo4jBackend")
            .field("url", &self.url)
            .finish()
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
        let graph = self
            .graph
            .as_ref()
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
        let graph = self
            .graph
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let query = neo4rs::query(&format!("MATCH (n:{type_name} {{_id: $id}}) RETURN n"))
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
                    record
                        .fields
                        .insert(key.to_string(), crate::value::Value::String(val));
                }
            }

            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    pub async fn query(&self, type_name: &str, filter: &QueryFilter) -> StorageResult<Vec<Record>> {
        let graph = self
            .graph
            .as_ref()
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
                    format!(
                        "n.{} {} '{}'",
                        c.field,
                        op,
                        value_to_cypher_string(&c.value)
                    )
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
                    record
                        .fields
                        .insert(key.to_string(), crate::value::Value::String(val));
                }
            }
            records.push(record);
        }

        Ok(records)
    }

    /// If the record has subject, predicate, and object fields,
    /// create (or merge) a relationship triple in the graph:
    ///   (subject_entity)-[:PREDICATE]->(object_entity)
    /// Both subject and object become Entity nodes; the record itself
    /// is linked to both via HAS_FACT edges.
    pub async fn store_triple(&self, record: &Record) -> StorageResult<()> {
        let graph = self
            .graph
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let subject = match record.fields.get("subject") {
            Some(crate::value::Value::String(s)) if !s.is_empty() => s.clone(),
            _ => return Ok(()),
        };
        let predicate = match record.fields.get("predicate") {
            Some(crate::value::Value::String(s)) if !s.is_empty() => s.clone(),
            _ => return Ok(()),
        };
        let object = match record.fields.get("object") {
            Some(crate::value::Value::String(s)) if !s.is_empty() => s.clone(),
            _ => return Ok(()),
        };

        let rel_type = predicate
            .to_uppercase()
            .replace(' ', "_")
            .replace('-', "_");

        let cypher = format!(
            "MERGE (s:Entity {{name: $subject}}) \
             MERGE (o:Entity {{name: $object}}) \
             MERGE (s)-[r:{rel_type}]->(o) \
             SET r.predicate = $predicate \
             WITH s, o \
             MATCH (f:{} {{_id: $fact_id}}) \
             MERGE (s)-[:HAS_FACT]->(f) \
             MERGE (o)-[:HAS_FACT]->(f)",
            record.type_name
        );

        let query = neo4rs::query(&cypher)
            .param("subject", subject)
            .param("object", object)
            .param("predicate", predicate)
            .param("fact_id", record.id.clone());

        graph
            .run(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        tracing::debug!(
            type_name = %record.type_name,
            id = %record.id,
            "stored graph triple"
        );

        Ok(())
    }

    /// Find record IDs connected to the input entity within N hops.
    /// Searches Entity nodes whose name contains the input, then
    /// traverses up to `hops` relationship levels to find connected
    /// fact nodes.
    pub async fn graph_match_ids(
        &self,
        type_name: &str,
        input: &str,
        hops: usize,
    ) -> StorageResult<std::collections::HashSet<String>> {
        let graph = self
            .graph
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let cypher = format!(
            "MATCH (e:Entity) \
             WHERE toLower(e.name) CONTAINS toLower($input) \
             MATCH (e)-[*1..{hops}]-(related) \
             WHERE related:{type_name} \
             RETURN DISTINCT related._id AS id"
        );

        let query = neo4rs::query(&cypher).param("input", input.to_string());

        let mut result = graph
            .execute(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        let mut ids = std::collections::HashSet::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?
        {
            if let Ok(id) = row.get::<String>("id") {
                ids.insert(id);
            }
        }

        Ok(ids)
    }

    /// Execute a raw Cypher query and collect returned `name` or `_id` values
    /// as a set of IDs for filtering.
    pub async fn cypher_query_ids(
        &self,
        cypher: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> StorageResult<std::collections::HashSet<String>> {
        let graph = self
            .graph
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected("neo4j".into()))?;

        let mut query = neo4rs::query(cypher);
        for (k, v) in params {
            query = query.param(k.as_str(), v.clone());
        }

        let mut result = graph
            .execute(query)
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?;

        let mut ids = std::collections::HashSet::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| StorageError::Neo4j(e.to_string()))?
        {
            if let Ok(id) = row.get::<String>("_id") {
                ids.insert(id);
            } else if let Ok(name) = row.get::<String>("name") {
                ids.insert(name);
            } else if let Ok(id) = row.get::<String>("id") {
                ids.insert(id);
            }
        }

        Ok(ids)
    }

    pub async fn update(&self, record: &Record) -> StorageResult<()> {
        self.store(record).await
    }

    pub async fn delete(&self, type_name: &str, id: &str) -> StorageResult<()> {
        let graph = self
            .graph
            .as_ref()
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
