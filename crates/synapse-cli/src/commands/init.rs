use std::fs;
use std::path::Path;

pub fn run() -> anyhow::Result<()> {
    println!("Creating synapse project...");

    // Create .synapse directory
    fs::create_dir_all(".synapse")?;
    fs::write(
        ".synapse/state.json",
        serde_json::json!({"version": "0.1.0", "runtime_pid": null}).to_string(),
    )?;
    println!("  ✓ Created .synapse/ (local state directory)");

    // Create synapse.toml
    if !Path::new("synapse.toml").exists() {
        fs::write(
            "synapse.toml",
            r#"[project]
name = "my-agent-memory"
version = "0.1.0"

[runtime]
host = "localhost"
port = 8080
log_level = "info"

[storage]
sqlite_path = "./data/synapse.db"
# qdrant_url = "http://localhost:6333"
# neo4j_url = "bolt://localhost:7687"
"#,
        )?;
        println!("  ✓ Created synapse.toml (project config)");
    }

    // Create starter .mnm file
    if !Path::new("synapse.mnm").exists() {
        fs::write(
            "synapse.mnm",
            r#"# Synapse Memory Configuration
# Edit this file to define your agent's memory system.

config {
    storage: sqlite("./data/synapse.db")
    embedding: openai("text-embedding-3-small")
}

memory Note {
    content: string
    created_at: timestamp
}

on save(content: string) {
    store(Note {
        content: content,
        created_at: now()
    })
}

query GetAll(): Note[] {
    from Note
    order by created_at desc
}
"#,
        )?;
        println!("  ✓ Created synapse.mnm (starter template)");
    }

    // Create data directory
    fs::create_dir_all("data")?;

    println!("\nProject initialized! Edit synapse.mnm and run `synapse apply` to start.");
    Ok(())
}
