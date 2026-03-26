use std::fs;
use std::path::Path;

use synapse_dsl::ast::{ChannelDef, Item};
use synapse_runtime::config::{GraphConfig, RuntimeConfig, VectorConfig};
use synapse_runtime::llm::{EmbeddingClient, LlmClient};
use synapse_runtime::storage::sqlite::SqliteBackend;
use synapse_runtime::storage::{GraphBackendKind, StorageBackend, VectorBackendKind};
use synapse_runtime::{docker, Runtime, StorageManager};

async fn connect_vector_backend(
    name: &str,
    cfg: &VectorConfig,
    data_dir: &Path,
) -> anyhow::Result<VectorBackendKind> {
    match cfg {
        VectorConfig::Auto { backend } => {
            docker::ensure_docker_available()?;
            match backend.as_str() {
                "qdrant" => {
                    let url = docker::ensure_qdrant(data_dir).await?;
                    let vb = synapse_runtime::storage::qdrant::QdrantBackend::connect(&url).await?;
                    println!("  ✓ Qdrant [{name}] auto-started at {url}");
                    Ok(VectorBackendKind::Qdrant(vb))
                }
                "weaviate" => {
                    let url = docker::ensure_weaviate(data_dir, name).await?;
                    let vb =
                        synapse_runtime::storage::weaviate::WeaviateBackend::connect(&url).await?;
                    println!("  ✓ Weaviate [{name}] auto-started at {url}");
                    Ok(VectorBackendKind::Weaviate(vb))
                }
                "chromadb" | "chroma" => {
                    let url = docker::ensure_chromadb(data_dir, name).await?;
                    let vb =
                        synapse_runtime::storage::chromadb::ChromaDBBackend::connect(&url).await?;
                    println!("  ✓ ChromaDB [{name}] auto-started at {url}");
                    Ok(VectorBackendKind::ChromaDB(vb))
                }
                other => anyhow::bail!("unknown vector backend for auto: {other}"),
            }
        }
        VectorConfig::External { backend, url } => match backend.as_str() {
            "qdrant" => {
                let vb = synapse_runtime::storage::qdrant::QdrantBackend::connect(url).await?;
                println!("  ✓ Qdrant [{name}] connected at {url}");
                Ok(VectorBackendKind::Qdrant(vb))
            }
            "pinecone" => {
                let vb = synapse_runtime::storage::pinecone::PineconeBackend::connect(url).await?;
                println!("  ✓ Pinecone [{name}] connected at {url}");
                Ok(VectorBackendKind::Pinecone(vb))
            }
            "weaviate" => {
                let vb = synapse_runtime::storage::weaviate::WeaviateBackend::connect(url).await?;
                println!("  ✓ Weaviate [{name}] connected at {url}");
                Ok(VectorBackendKind::Weaviate(vb))
            }
            "chromadb" | "chroma" => {
                let vb = synapse_runtime::storage::chromadb::ChromaDBBackend::connect(url).await?;
                println!("  ✓ ChromaDB [{name}] connected at {url}");
                Ok(VectorBackendKind::ChromaDB(vb))
            }
            other => anyhow::bail!("unknown vector backend: {other}"),
        },
    }
}

async fn connect_graph_backend(
    name: &str,
    cfg: &GraphConfig,
    data_dir: &Path,
) -> anyhow::Result<GraphBackendKind> {
    match cfg {
        GraphConfig::Auto { backend } => {
            docker::ensure_docker_available()?;
            match backend.as_str() {
                "neo4j" => {
                    let url = docker::ensure_neo4j(data_dir).await?;
                    let gb = synapse_runtime::storage::neo4j::Neo4jBackend::connect(&url).await?;
                    println!("  ✓ Neo4j [{name}] auto-started at {url}");
                    Ok(GraphBackendKind::Neo4j(gb))
                }
                "memgraph" => {
                    let url = docker::ensure_memgraph(data_dir, name).await?;
                    let gb =
                        synapse_runtime::storage::memgraph::MemgraphBackend::connect(&url).await?;
                    println!("  ✓ Memgraph [{name}] auto-started at {url}");
                    Ok(GraphBackendKind::Memgraph(gb))
                }
                "arangodb" | "arango" => {
                    let url = docker::ensure_arangodb(data_dir, name).await?;
                    let gb =
                        synapse_runtime::storage::arangodb::ArangoDBBackend::connect(&url).await?;
                    println!("  ✓ ArangoDB [{name}] auto-started at {url}");
                    Ok(GraphBackendKind::ArangoDB(gb))
                }
                "surrealdb" | "surreal" => {
                    let url = docker::ensure_surrealdb(data_dir, name).await?;
                    let gb = synapse_runtime::storage::surrealdb::SurrealDBBackend::connect(&url)
                        .await?;
                    println!("  ✓ SurrealDB [{name}] auto-started at {url}");
                    Ok(GraphBackendKind::SurrealDB(gb))
                }
                other => anyhow::bail!("unknown graph backend for auto: {other}"),
            }
        }
        GraphConfig::External { backend, url } => match backend.as_str() {
            "neo4j" => {
                let gb = synapse_runtime::storage::neo4j::Neo4jBackend::connect(url).await?;
                println!("  ✓ Neo4j [{name}] connected at {url}");
                Ok(GraphBackendKind::Neo4j(gb))
            }
            "memgraph" => {
                let gb = synapse_runtime::storage::memgraph::MemgraphBackend::connect(url).await?;
                println!("  ✓ Memgraph [{name}] connected at {url}");
                Ok(GraphBackendKind::Memgraph(gb))
            }
            "arangodb" | "arango" => {
                let gb = synapse_runtime::storage::arangodb::ArangoDBBackend::connect(url).await?;
                println!("  ✓ ArangoDB [{name}] connected at {url}");
                Ok(GraphBackendKind::ArangoDB(gb))
            }
            "surrealdb" | "surreal" => {
                let gb =
                    synapse_runtime::storage::surrealdb::SurrealDBBackend::connect(url).await?;
                println!("  ✓ SurrealDB [{name}] connected at {url}");
                Ok(GraphBackendKind::SurrealDB(gb))
            }
            other => anyhow::bail!("unknown graph backend: {other}"),
        },
    }
}

pub async fn run(file: &str, port: Option<u16>, daemon: bool) -> anyhow::Result<()> {
    println!("Applying {file}...");

    let source = fs::read_to_string(file)?;
    let program = synapse_dsl::parser::parse(&source)?;
    synapse_dsl::typeck::check(&program)?;
    println!("  ✓ Compiled successfully");

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

    let mut storage = StorageManager::new();

    if let Some(ref sc) = config.storage {
        let sqlite = SqliteBackend::open(&sc.url)?;
        storage.relational = Some(StorageBackend::Sqlite(sqlite));
        println!("  ✓ Connected to sqlite ({})", sc.url);
    }

    // Connect vector backends
    for (name, vcfg) in &config.vectors {
        match connect_vector_backend(name, vcfg, data_dir).await {
            Ok(vb) => {
                storage.vectors.insert(name.clone(), vb);
            }
            Err(e) => {
                tracing::error!(error = %e, backend = %name, "failed to connect vector backend");
                eprintln!("  ✗ Vector backend [{name}] failed: {e}");
            }
        }
    }

    // Connect graph backends
    for (name, gcfg) in &config.graphs {
        match connect_graph_backend(name, gcfg, data_dir).await {
            Ok(gb) => {
                storage.graphs.insert(name.clone(), gb);
            }
            Err(e) => {
                tracing::error!(error = %e, backend = %name, "failed to connect graph backend");
                eprintln!("  ✗ Graph backend [{name}] failed: {e}");
            }
        }
    }

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

    // Wire embedder to all vector backends
    if let Some(ref emb) = embedder {
        for vb in storage.vectors.values_mut() {
            vb.set_embedder(emb.clone());
        }
    }

    // Collect channel definitions from the program
    fn collect_channel_defs(items: &[Item]) -> Vec<ChannelDef> {
        let mut defs = Vec::new();
        for item in items {
            match item {
                Item::Channel(ch) => defs.push(ch.clone()),
                Item::Namespace(ns) => defs.extend(collect_channel_defs(&ns.items)),
                _ => {}
            }
        }
        defs
    }

    let channel_defs = collect_channel_defs(&program.items);
    for ch_def in &channel_defs {
        config.add_channel_from_def(ch_def);
    }

    let runtime = Runtime::new(program, storage, llm, embedder).with_source_file(file);
    runtime.init_storage().await?;

    // Setup and start channels
    let mut channel_manager = synapse_runtime::channels::setup_channels(&config.channels).await?;
    let channel_rx = channel_manager.take_receiver();
    synapse_runtime::channels::start_channel_polling(&mut channel_manager, &config.channels);

    let scheduler = synapse_runtime::interpreter::policy::PolicyScheduler::from_program(
        &runtime.program,
        runtime.storage.clone(),
        runtime.llm.clone(),
        runtime.embedder.clone(),
        runtime.handlers.clone(),
        runtime.extern_fns.clone(),
        runtime.queries.clone(),
        runtime.updates.clone(),
        runtime.memories.clone(),
    );
    let _policy_handles = scheduler.start();

    let channel_count = config.channels.len();

    let addr = format!("{}:{}", config.host, config.port);
    println!("  ✓ Runtime listening on {addr}");

    if channel_count > 0 {
        println!("  ✓ {} channel(s) active and polling", channel_count);
    }

    if daemon {
        println!("  Running in background...");
    } else {
        println!("\n  Press Ctrl+C to stop.\n");
    }

    let state = serde_json::json!({
        "pid": std::process::id(),
        "addr": addr,
        "file": file,
    });
    let _ = fs::create_dir_all(".synapse");
    let _ = fs::write(".synapse/state.json", state.to_string());

    // Build router with shared runtime state
    let router = synapse_runtime::server::build_router(runtime);

    // Spawn channel event dispatcher if there are channels
    // Note: The dispatcher connects to the runtime via its own event handlers
    if let Some(rx) = channel_rx {
        if channel_count > 0 {
            tracing::info!("channel event dispatcher started");
            tokio::spawn(async move {
                let mut rx = rx;
                while let Some(event) = rx.recv().await {
                    tracing::info!(
                        channel = %event.channel_name,
                        event_type = %event.event_type.as_str(),
                        source = %event.source,
                        "received channel event"
                    );
                }
            });
        }
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Synapse runtime listening on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
