use std::fs;
use std::path::Path;

use synapse_core::ast::Item;
use synapse_runtime::config::{GraphConfig, RuntimeConfig, VectorConfig};
use synapse_runtime::llm::{EmbeddingClient, LlmClient};
use synapse_runtime::storage::neo4j::Neo4jBackend;
use synapse_runtime::storage::qdrant::QdrantBackend;
use synapse_runtime::storage::sqlite::SqliteBackend;
use synapse_runtime::storage::StorageBackend;
use synapse_runtime::{docker, Runtime, StorageManager};

pub async fn run(file: &str, port: Option<u16>, daemon: bool) -> anyhow::Result<()> {
    println!("Applying {file}...");

    let source = fs::read_to_string(file)?;
    let program = synapse_core::parser::parse(&source)?;
    synapse_core::typeck::check(&program)?;
    println!("  ✓ Compiled successfully");

    // Extract config
    let mut config = RuntimeConfig::default();
    for item in &program.items {
        if let Item::Config(cfg) = item {
            config = RuntimeConfig::from_config_block(cfg);
            break;
        }
    }

    if let Some(p) = port {
        config.port = p;
    }

    let data_dir = Path::new(".synapse");

    // Set up storage
    let mut storage = StorageManager::new();

    if let Some(ref sc) = config.storage {
        let sqlite = SqliteBackend::open(&sc.url)?;
        storage.relational = Some(StorageBackend::Sqlite(sqlite));
        println!("  ✓ Connected to sqlite ({})", sc.url);
    }

    match &config.vector {
        Some(VectorConfig::Auto) => {
            docker::ensure_docker_available()?;
            let url = docker::ensure_qdrant(data_dir).await?;
            let qdrant = QdrantBackend::connect(&url).await?;
            storage.vector = Some(StorageBackend::Qdrant(qdrant));
            println!("  ✓ Qdrant auto-started at {url}");
        }
        Some(VectorConfig::External { url, .. }) => {
            let qdrant = QdrantBackend::connect(url).await?;
            storage.vector = Some(StorageBackend::Qdrant(qdrant));
            println!("  ✓ Connected to Qdrant ({url})");
        }
        None => {}
    }

    match &config.graph {
        Some(GraphConfig::Auto) => {
            docker::ensure_docker_available()?;
            let url = docker::ensure_neo4j(data_dir).await?;
            let neo4j = Neo4jBackend::connect(&url).await?;
            storage.graph = Some(StorageBackend::Neo4j(neo4j));
            println!("  ✓ Neo4j auto-started at {url}");
        }
        Some(GraphConfig::External { url, .. }) => {
            let neo4j = Neo4jBackend::connect(url).await?;
            storage.graph = Some(StorageBackend::Neo4j(neo4j));
            println!("  ✓ Connected to Neo4j ({url})");
        }
        None => {}
    }

    // Build LLM client from extractor config (if present)
    let llm = match &config.extractor {
        Some(ext_cfg) => {
            let client = LlmClient::from_config(ext_cfg)?;
            println!(
                "  ✓ LLM extractor configured ({}/{})",
                ext_cfg.provider, ext_cfg.model
            );
            Some(client)
        }
        None => None,
    };

    // Build embedding client (if configured), shared between storage and runtime
    let embedder = match &config.embedding {
        Some(emb_cfg) => {
            let client = EmbeddingClient::from_config(emb_cfg)?;
            println!(
                "  ✓ Embedding model configured ({}/{})",
                emb_cfg.provider, emb_cfg.model
            );
            Some(std::sync::Arc::new(client))
        }
        None => None,
    };

    storage.embedder = embedder.clone();

    if let Some(ref emb) = embedder {
        if let Some(StorageBackend::Qdrant(ref mut qdrant)) = storage.vector {
            qdrant.set_embedder(emb.clone());
        }
    }

    // Build runtime
    let runtime = Runtime::new(program, storage, llm, embedder).with_source_file(file);
    runtime.init_storage().await?;

    // Start policy scheduler
    let scheduler = synapse_runtime::interpreter::policy::PolicyScheduler::from_program(
        &runtime.program,
        runtime.storage.clone(),
        runtime.llm.clone(),
        runtime.embedder.clone(),
        runtime.handlers.clone(),
        runtime.extern_fns.clone(),
    );
    let _policy_handles = scheduler.start();

    let addr = format!("{}:{}", config.host, config.port);
    println!("  ✓ Runtime listening on {addr}");

    if daemon {
        println!("  Running in background...");
    } else {
        println!("\n  Press Ctrl+C to stop.\n");
    }

    // Save state
    let state = serde_json::json!({
        "pid": std::process::id(),
        "addr": addr,
        "file": file,
    });
    let _ = fs::create_dir_all(".synapse");
    let _ = fs::write(".synapse/state.json", state.to_string());

    synapse_runtime::server::serve(runtime, &addr).await?;

    Ok(())
}
