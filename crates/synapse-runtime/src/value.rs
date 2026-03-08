use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime value — the universal representation of data flowing through
/// handlers, queries, and storage backends. Uses enum dispatch (no Box<dyn>).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Timestamp(DateTime<Utc>),
    Array(Vec<Value>),
    Record(Record),
}

/// A typed record — represents a memory instance with named fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    #[serde(rename = "_type")]
    pub type_name: String,
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

impl Record {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            id: uuid::Uuid::new_v4().to_string(),
            fields: HashMap::new(),
        }
    }

    pub fn with_field(mut self, name: impl Into<String>, value: Value) -> Self {
        self.fields.insert(name.into(), value);
        self
    }

    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.get(field)
    }

    pub fn set(&mut self, field: &str, value: Value) {
        self.fields.insert(field.to_string(), value);
    }
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_record(&self) -> Option<&Record> {
        match self {
            Value::Record(r) => Some(r),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            _ => true,
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(a) => {
                Value::Array(a.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(o) => {
                let mut fields: HashMap<String, Value> =
                    o.into_iter().map(|(k, v)| (k, Value::from(v))).collect();
                let type_name = fields
                    .remove("_type")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                let id = fields
                    .remove("_id")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                Value::Record(Record {
                    type_name,
                    id,
                    fields,
                })
            }
        }
    }
}

impl From<Value> for serde_json::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Int(n) => serde_json::json!(n),
            Value::Float(f) => serde_json::json!(f),
            Value::String(s) => serde_json::Value::String(s),
            Value::Timestamp(t) => serde_json::Value::String(t.to_rfc3339()),
            Value::Array(a) => {
                serde_json::Value::Array(a.into_iter().map(serde_json::Value::from).collect())
            }
            Value::Record(r) => {
                let mut map = serde_json::Map::new();
                map.insert("_type".into(), serde_json::json!(r.type_name));
                map.insert("_id".into(), serde_json::json!(r.id));
                for (k, v) in r.fields {
                    map.insert(k, serde_json::Value::from(v));
                }
                serde_json::Value::Object(map)
            }
        }
    }
}
