pub mod handler;
pub mod policy;
pub mod query;
pub mod update;

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use synapse_dsl::ast::*;
use tokio::sync::RwLock;

use crate::llm::{EmbeddingClient, LlmClient};
use crate::storage::StorageManager;
use crate::value::Value;

/// The runtime interpreter — executes DSL constructs against storage.
/// Uses Arc<RwLock<>> for interior mutability shared across async tasks.
pub struct Runtime {
    pub storage: Arc<StorageManager>,
    pub llm: Option<Arc<LlmClient>>,
    pub embedder: Option<Arc<EmbeddingClient>>,
    pub program: Program,
    /// Registered handler definitions indexed by event name
    pub handlers: Arc<HashMap<String, HandlerDef>>,
    /// Registered query definitions indexed by query name
    pub queries: Arc<HashMap<String, QueryDef>>,
    /// Registered update definitions indexed by memory type
    pub updates: Arc<HashMap<String, UpdateDef>>,
    /// Memory definitions: type_name -> full MemoryDef (fields, indexes, invariants)
    pub memories: Arc<HashMap<String, MemoryDef>>,
    pub extern_fns: Arc<HashMap<String, ExternFnDef>>,
    /// Channel definitions indexed by channel name
    pub channels: Arc<HashMap<String, ChannelDef>>,
    /// Path to the .mnm source file (for hot reload)
    pub source_file: Option<String>,
    /// Runtime state: counters, stats, etc.
    pub stats: Arc<RwLock<RuntimeStats>>,
}

#[derive(Debug, Default)]
pub struct RuntimeStats {
    pub events_processed: u64,
    pub queries_executed: u64,
    pub records_stored: u64,
    pub started_at: Option<chrono::DateTime<Utc>>,
}

impl Runtime {
    pub fn new(
        program: Program,
        storage: StorageManager,
        llm: Option<LlmClient>,
        embedder: Option<Arc<EmbeddingClient>>,
    ) -> Self {
        let mut handlers = HashMap::new();
        let mut queries = HashMap::new();
        let mut updates = HashMap::new();
        let mut memories = HashMap::new();
        let mut extern_fns = HashMap::new();
        let mut channels = HashMap::new();

        collect_definitions(
            &program.items,
            &mut handlers,
            &mut queries,
            &mut updates,
            &mut memories,
            &mut extern_fns,
            &mut channels,
        );

        Self {
            storage: Arc::new(storage),
            llm: llm.map(Arc::new),
            embedder,
            program,
            handlers: Arc::new(handlers),
            queries: Arc::new(queries),
            updates: Arc::new(updates),
            memories: Arc::new(memories),
            extern_fns: Arc::new(extern_fns),
            channels: Arc::new(channels),
            source_file: None,
            stats: Arc::new(RwLock::new(RuntimeStats {
                started_at: Some(Utc::now()),
                ..Default::default()
            })),
        }
    }

    /// Set the source file path (used for hot reload).
    pub fn with_source_file(mut self, path: &str) -> Self {
        self.source_file = Some(path.to_string());
        self
    }

    /// Hot-reload: re-read the source file, re-parse, and update definitions in place.
    pub fn reload(&mut self) -> anyhow::Result<()> {
        let path = self
            .source_file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no source file configured for reload"))?;

        let source = std::fs::read_to_string(path)?;
        let program = synapse_dsl::parser::parse(&source)?;
        synapse_dsl::typeck::check(&program)?;

        let mut handlers = HashMap::new();
        let mut queries = HashMap::new();
        let mut updates = HashMap::new();
        let mut memories = HashMap::new();
        let mut extern_fns = HashMap::new();
        let mut channels = HashMap::new();

        collect_definitions(
            &program.items,
            &mut handlers,
            &mut queries,
            &mut updates,
            &mut memories,
            &mut extern_fns,
            &mut channels,
        );

        self.program = program;
        self.handlers = Arc::new(handlers);
        self.queries = Arc::new(queries);
        self.updates = Arc::new(updates);
        self.memories = Arc::new(memories);
        self.extern_fns = Arc::new(extern_fns);
        self.channels = Arc::new(channels);

        tracing::info!(file = %path, "runtime reloaded successfully");
        Ok(())
    }

    /// Initialize storage tables for all memory definitions
    pub async fn init_storage(&self) -> anyhow::Result<()> {
        for (name, mem) in self.memories.as_ref() {
            let field_specs: Vec<(String, String)> = mem
                .fields
                .iter()
                .map(|f| (f.name.clone(), type_to_storage_string(&f.ty)))
                .collect();
            self.storage
                .ensure_table(name, &field_specs, &mem.indexes)
                .await?;
        }
        Ok(())
    }

    /// Emit an event, triggering the matching handler
    pub async fn emit(
        &self,
        event: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let handler = self
            .handlers
            .get(event)
            .ok_or_else(|| anyhow::anyhow!("unknown event: {event}"))?
            .clone();

        let mut env = ExecEnv::new(
            self.storage.clone(),
            self.llm.clone(),
            self.embedder.clone(),
            self.handlers.clone(),
            self.extern_fns.clone(),
        )
        .with_queries(self.queries.clone())
        .with_updates(self.updates.clone())
        .with_memories(self.memories.clone());

        // Bind handler parameters from the payload
        if let serde_json::Value::Object(map) = &payload {
            for param in &handler.params {
                if let Some(val) = map.get(&param.name) {
                    env.set(&param.name, Value::from(val.clone()));
                }
            }
        }

        // Execute handler body
        handler::exec_stmts(&mut env, &handler.body).await?;

        self.stats.write().await.events_processed += 1;

        Ok(serde_json::json!({
            "success": true,
            "event": event,
            "stored": env.stored_count
        }))
    }

    /// Execute a named query
    pub async fn query(
        &self,
        name: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let query_def = self
            .queries
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown query: {name}"))?
            .clone();

        let mut env = ExecEnv::new(
            self.storage.clone(),
            self.llm.clone(),
            self.embedder.clone(),
            self.handlers.clone(),
            self.extern_fns.clone(),
        )
        .with_queries(self.queries.clone())
        .with_updates(self.updates.clone())
        .with_memories(self.memories.clone());

        // Bind query parameters
        if let serde_json::Value::Object(map) = &params {
            for param in &query_def.params {
                if let Some(val) = map.get(&param.name) {
                    env.set(&param.name, Value::from(val.clone()));
                }
            }
        }

        let results = query::exec_query(&mut env, &query_def).await?;

        self.stats.write().await.queries_executed += 1;

        let json_results: Vec<serde_json::Value> =
            results.into_iter().map(serde_json::Value::from).collect();

        Ok(serde_json::json!(json_results))
    }

    /// Get handler names
    pub fn handler_names(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Get query names
    pub fn query_names(&self) -> Vec<&str> {
        self.queries.keys().map(|s| s.as_str()).collect()
    }

    /// Get memory names
    pub fn memory_names(&self) -> Vec<&str> {
        self.memories.keys().map(|s| s.as_str()).collect()
    }

    /// Get channel names
    pub fn channel_names(&self) -> Vec<&str> {
        self.channels.keys().map(|s| s.as_str()).collect()
    }
}

/// Execution environment — holds variable bindings and storage access for
/// a single handler/query execution.
pub struct ExecEnv {
    pub storage: Arc<StorageManager>,
    pub llm: Option<Arc<LlmClient>>,
    pub embedder: Option<Arc<EmbeddingClient>>,
    pub handlers: Arc<HashMap<String, HandlerDef>>,
    pub extern_fns: Arc<HashMap<String, ExternFnDef>>,
    pub queries: Arc<HashMap<String, QueryDef>>,
    pub updates: Arc<HashMap<String, UpdateDef>>,
    pub memories: Arc<HashMap<String, MemoryDef>>,
    scopes: Vec<HashMap<String, Value>>,
    pub stored_count: u64,
}

impl ExecEnv {
    pub fn new(
        storage: Arc<StorageManager>,
        llm: Option<Arc<LlmClient>>,
        embedder: Option<Arc<EmbeddingClient>>,
        handlers: Arc<HashMap<String, HandlerDef>>,
        extern_fns: Arc<HashMap<String, ExternFnDef>>,
    ) -> Self {
        Self {
            storage,
            llm,
            embedder,
            handlers,
            extern_fns,
            queries: Arc::new(HashMap::new()),
            updates: Arc::new(HashMap::new()),
            memories: Arc::new(HashMap::new()),
            scopes: vec![HashMap::new()],
            stored_count: 0,
        }
    }

    pub fn with_queries(mut self, queries: Arc<HashMap<String, QueryDef>>) -> Self {
        self.queries = queries;
        self
    }

    pub fn with_updates(mut self, updates: Arc<HashMap<String, UpdateDef>>) -> Self {
        self.updates = updates;
        self
    }

    pub fn with_memories(mut self, memories: Arc<HashMap<String, MemoryDef>>) -> Self {
        self.memories = memories;
        self
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn set(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    pub fn get(&self, name: &str) -> Value {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return val.clone();
            }
        }
        Value::Null
    }
}

fn collect_definitions(
    items: &[Item],
    handlers: &mut HashMap<String, HandlerDef>,
    queries: &mut HashMap<String, QueryDef>,
    updates: &mut HashMap<String, UpdateDef>,
    memories: &mut HashMap<String, MemoryDef>,
    extern_fns: &mut HashMap<String, ExternFnDef>,
    channels: &mut HashMap<String, ChannelDef>,
) {
    for item in items {
        match item {
            Item::Handler(h) => {
                handlers.insert(h.event.clone(), h.clone());
            }
            Item::Query(q) => {
                queries.insert(q.name.clone(), q.clone());
            }
            Item::Update(u) => {
                updates.insert(u.target.clone(), u.clone());
            }
            Item::Memory(m) => {
                memories.insert(m.name.clone(), m.clone());
            }
            Item::ExternFn(ef) => {
                extern_fns.insert(ef.name.clone(), ef.clone());
            }
            Item::Channel(ch) => {
                channels.insert(ch.name.clone(), ch.clone());
            }
            Item::Namespace(ns) => {
                collect_definitions(
                    &ns.items, handlers, queries, updates, memories, extern_fns, channels,
                );
            }
            _ => {}
        }
    }
}

fn type_to_storage_string(ty: &synapse_dsl::types::Type) -> String {
    match ty {
        synapse_dsl::types::Type::String => "string".into(),
        synapse_dsl::types::Type::Int => "int".into(),
        synapse_dsl::types::Type::Float | synapse_dsl::types::Type::BoundedFloat { .. } => {
            "float".into()
        }
        synapse_dsl::types::Type::Bool => "bool".into(),
        synapse_dsl::types::Type::Timestamp => "timestamp".into(),
        synapse_dsl::types::Type::Optional(inner) => type_to_storage_string(inner),
        synapse_dsl::types::Type::Array(_) => "string".into(), // JSON-serialized
        synapse_dsl::types::Type::Named(_) => "string".into(), // foreign key
    }
}
