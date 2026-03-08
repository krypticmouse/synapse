use std::fs;

use synapse_core::ast::Item;
use synapse_runtime::config::RuntimeConfig;
use synapse_runtime::storage::sqlite::SqliteBackend;
use synapse_runtime::storage::StorageBackend;
use synapse_runtime::{Runtime, StorageManager};

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

    // Set up storage
    let mut storage = StorageManager::new();

    if let Some(ref sc) = config.storage {
        let sqlite = SqliteBackend::open(&sc.url)?;
        storage.relational = Some(StorageBackend::Sqlite(sqlite));
        println!("  ✓ Connected to sqlite ({})", sc.url);
    }

    if let Some(ref vc) = config.vector {
        println!("  ✓ Qdrant configured at {} (connect on demand)", vc.url);
    }

    if let Some(ref gc) = config.graph {
        println!("  ✓ Neo4j configured at {} (connect on demand)", gc.url);
    }

    // Build runtime
    let runtime = Runtime::new(program, storage);
    runtime.init_storage().await?;

    // Start policy scheduler
    let scheduler = synapse_runtime::interpreter::policy::PolicyScheduler::from_program(
        &runtime.program,
        runtime.storage.clone(),
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
