use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};

const QDRANT_CONTAINER: &str = "synapse-qdrant";
const QDRANT_IMAGE: &str = "qdrant/qdrant";
const QDRANT_PORT: u16 = 6333;
const QDRANT_GRPC_PORT: u16 = 6334;

const NEO4J_CONTAINER: &str = "synapse-neo4j";
const NEO4J_IMAGE: &str = "neo4j:latest";
const NEO4J_BOLT_PORT: u16 = 7687;
const NEO4J_HTTP_PORT: u16 = 7474;

/// Verify that Docker is installed and the daemon is running.
pub fn ensure_docker_available() -> Result<()> {
    let output = Command::new("docker")
        .args(["info"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match output {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => bail!(
            "Docker daemon is not running. Start Docker and try again.\n\
             Synapse requires Docker to auto-manage Qdrant and Neo4j backends."
        ),
        Err(_) => bail!(
            "Docker is not installed. Install Docker and try again.\n\
             Synapse requires Docker to auto-manage Qdrant and Neo4j backends.\n\
             Install: https://docs.docker.com/get-docker/"
        ),
    }
}

#[derive(Debug, PartialEq)]
enum ContainerState {
    Running,
    Stopped,
    NotFound,
}

fn inspect_container(name: &str) -> ContainerState {
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.State.Running}}", name])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim() == "true" {
                ContainerState::Running
            } else {
                ContainerState::Stopped
            }
        }
        _ => ContainerState::NotFound,
    }
}

fn start_container(name: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["start", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("failed to start Docker container")?;

    if !status.success() {
        bail!("failed to start container '{name}'");
    }
    Ok(())
}

/// Ensure a Qdrant container is running, creating it if necessary.
/// Data is persisted in `{data_dir}/qdrant_storage`.
/// Returns the HTTP URL to connect to.
pub async fn ensure_qdrant(data_dir: &Path) -> Result<String> {
    let storage_dir = data_dir.join("qdrant_storage");
    std::fs::create_dir_all(&storage_dir).context("failed to create Qdrant storage directory")?;

    let storage_path = storage_dir
        .canonicalize()
        .unwrap_or_else(|_| storage_dir.clone());

    match inspect_container(QDRANT_CONTAINER) {
        ContainerState::Running => {
            tracing::info!("Qdrant container already running");
        }
        ContainerState::Stopped => {
            tracing::info!("Starting existing Qdrant container");
            start_container(QDRANT_CONTAINER)?;
        }
        ContainerState::NotFound => {
            tracing::info!("Creating new Qdrant container");
            let status = Command::new("docker")
                .args([
                    "run",
                    "-d",
                    "--name",
                    QDRANT_CONTAINER,
                    "-p",
                    &format!("{QDRANT_PORT}:{QDRANT_PORT}"),
                    "-p",
                    &format!("{QDRANT_GRPC_PORT}:{QDRANT_GRPC_PORT}"),
                    "-v",
                    &format!("{}:/qdrant/storage", storage_path.display()),
                    QDRANT_IMAGE,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
                .context("failed to run Qdrant container")?;

            if !status.success() {
                bail!(
                    "Failed to create Qdrant container. Is port {QDRANT_PORT} already in use?\n\
                     Try: docker rm -f {QDRANT_CONTAINER}"
                );
            }
        }
    }

    let url = format!("http://localhost:{QDRANT_GRPC_PORT}");
    wait_for_port(QDRANT_GRPC_PORT, Duration::from_secs(30)).await?;
    Ok(url)
}

/// Ensure a Neo4j container is running, creating it if necessary.
/// Data is persisted in `{data_dir}/neo4j_data`.
/// Returns the Bolt URL to connect to.
pub async fn ensure_neo4j(data_dir: &Path) -> Result<String> {
    let data_path = data_dir.join("neo4j_data");
    std::fs::create_dir_all(&data_path).context("failed to create Neo4j data directory")?;

    let data_abs = data_path
        .canonicalize()
        .unwrap_or_else(|_| data_path.clone());

    match inspect_container(NEO4J_CONTAINER) {
        ContainerState::Running => {
            tracing::info!("Neo4j container already running");
        }
        ContainerState::Stopped => {
            tracing::info!("Starting existing Neo4j container");
            start_container(NEO4J_CONTAINER)?;
        }
        ContainerState::NotFound => {
            tracing::info!("Creating new Neo4j container");
            let status = Command::new("docker")
                .args([
                    "run",
                    "-d",
                    "--name",
                    NEO4J_CONTAINER,
                    "-p",
                    &format!("{NEO4J_BOLT_PORT}:{NEO4J_BOLT_PORT}"),
                    "-p",
                    &format!("{NEO4J_HTTP_PORT}:{NEO4J_HTTP_PORT}"),
                    "-e",
                    "NEO4J_AUTH=none",
                    "-v",
                    &format!("{}:/data", data_abs.display()),
                    NEO4J_IMAGE,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
                .context("failed to run Neo4j container")?;

            if !status.success() {
                bail!(
                    "Failed to create Neo4j container. Is port {NEO4J_BOLT_PORT} already in use?\n\
                     Try: docker rm -f {NEO4J_CONTAINER}"
                );
            }
        }
    }

    wait_for_port(NEO4J_BOLT_PORT, Duration::from_secs(60)).await?;

    let bolt_url = format!("bolt://localhost:{NEO4J_BOLT_PORT}");
    Ok(bolt_url)
}

// ═══════════════════════════════════════════════════════════════
// ADDITIONAL BACKENDS
// ═══════════════════════════════════════════════════════════════

const WEAVIATE_CONTAINER: &str = "synapse-weaviate";
const WEAVIATE_IMAGE: &str = "semitechnologies/weaviate:latest";
const WEAVIATE_PORT: u16 = 8090;

const CHROMADB_CONTAINER: &str = "synapse-chromadb";
const CHROMADB_IMAGE: &str = "chromadb/chroma:latest";
const CHROMADB_PORT: u16 = 8091;

const MEMGRAPH_CONTAINER: &str = "synapse-memgraph";
const MEMGRAPH_IMAGE: &str = "memgraph/memgraph:latest";
const MEMGRAPH_BOLT_PORT: u16 = 7688;

const ARANGODB_CONTAINER: &str = "synapse-arangodb";
const ARANGODB_IMAGE: &str = "arangodb:latest";
const ARANGODB_PORT: u16 = 8529;

const SURREALDB_CONTAINER: &str = "synapse-surrealdb";
const SURREALDB_IMAGE: &str = "surrealdb/surrealdb:latest";
const SURREALDB_PORT: u16 = 8092;

/// Ensure a Weaviate container is running. Returns the HTTP URL.
pub async fn ensure_weaviate(data_dir: &Path, instance: &str) -> Result<String> {
    let container = format!("{WEAVIATE_CONTAINER}-{instance}");
    let storage_dir = data_dir.join(format!("weaviate_{instance}"));
    std::fs::create_dir_all(&storage_dir)?;
    let port = WEAVIATE_PORT + hash_offset(instance);

    match inspect_container(&container) {
        ContainerState::Running => {}
        ContainerState::Stopped => { start_container(&container)?; }
        ContainerState::NotFound => {
            let status = Command::new("docker")
                .args([
                    "run", "-d", "--name", &container,
                    "-p", &format!("{port}:8080"),
                    "-e", "AUTHENTICATION_ANONYMOUS_ACCESS_ENABLED=true",
                    "-e", "PERSISTENCE_DATA_PATH=/var/lib/weaviate",
                    "-v", &format!("{}:/var/lib/weaviate", storage_dir.canonicalize().unwrap_or(storage_dir).display()),
                    WEAVIATE_IMAGE,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()?;
            if !status.success() {
                bail!("Failed to create Weaviate container on port {port}");
            }
        }
    }

    wait_for_port(port, Duration::from_secs(30)).await?;
    Ok(format!("http://localhost:{port}"))
}

/// Ensure a ChromaDB container is running. Returns the HTTP URL.
pub async fn ensure_chromadb(data_dir: &Path, instance: &str) -> Result<String> {
    let container = format!("{CHROMADB_CONTAINER}-{instance}");
    let storage_dir = data_dir.join(format!("chromadb_{instance}"));
    std::fs::create_dir_all(&storage_dir)?;
    let port = CHROMADB_PORT + hash_offset(instance);

    match inspect_container(&container) {
        ContainerState::Running => {}
        ContainerState::Stopped => { start_container(&container)?; }
        ContainerState::NotFound => {
            let status = Command::new("docker")
                .args([
                    "run", "-d", "--name", &container,
                    "-p", &format!("{port}:8000"),
                    "-v", &format!("{}:/chroma/chroma", storage_dir.canonicalize().unwrap_or(storage_dir).display()),
                    CHROMADB_IMAGE,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()?;
            if !status.success() {
                bail!("Failed to create ChromaDB container on port {port}");
            }
        }
    }

    wait_for_port(port, Duration::from_secs(30)).await?;
    Ok(format!("http://localhost:{port}"))
}

/// Ensure a Memgraph container is running. Returns the Bolt URL.
pub async fn ensure_memgraph(data_dir: &Path, instance: &str) -> Result<String> {
    let container = format!("{MEMGRAPH_CONTAINER}-{instance}");
    let storage_dir = data_dir.join(format!("memgraph_{instance}"));
    std::fs::create_dir_all(&storage_dir)?;
    let port = MEMGRAPH_BOLT_PORT + hash_offset(instance);

    match inspect_container(&container) {
        ContainerState::Running => {}
        ContainerState::Stopped => { start_container(&container)?; }
        ContainerState::NotFound => {
            let status = Command::new("docker")
                .args([
                    "run", "-d", "--name", &container,
                    "-p", &format!("{port}:7687"),
                    "-v", &format!("{}:/var/lib/memgraph", storage_dir.canonicalize().unwrap_or(storage_dir).display()),
                    MEMGRAPH_IMAGE,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()?;
            if !status.success() {
                bail!("Failed to create Memgraph container on port {port}");
            }
        }
    }

    wait_for_port(port, Duration::from_secs(30)).await?;
    Ok(format!("bolt://localhost:{port}"))
}

/// Ensure an ArangoDB container is running. Returns the HTTP URL.
pub async fn ensure_arangodb(data_dir: &Path, instance: &str) -> Result<String> {
    let container = format!("{ARANGODB_CONTAINER}-{instance}");
    let storage_dir = data_dir.join(format!("arangodb_{instance}"));
    std::fs::create_dir_all(&storage_dir)?;
    let port = ARANGODB_PORT + hash_offset(instance);

    match inspect_container(&container) {
        ContainerState::Running => {}
        ContainerState::Stopped => { start_container(&container)?; }
        ContainerState::NotFound => {
            let status = Command::new("docker")
                .args([
                    "run", "-d", "--name", &container,
                    "-p", &format!("{port}:8529"),
                    "-e", "ARANGO_NO_AUTH=1",
                    "-v", &format!("{}:/var/lib/arangodb3", storage_dir.canonicalize().unwrap_or(storage_dir).display()),
                    ARANGODB_IMAGE,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()?;
            if !status.success() {
                bail!("Failed to create ArangoDB container on port {port}");
            }
        }
    }

    wait_for_port(port, Duration::from_secs(30)).await?;
    Ok(format!("http://localhost:{port}"))
}

/// Ensure a SurrealDB container is running. Returns the HTTP URL.
pub async fn ensure_surrealdb(data_dir: &Path, instance: &str) -> Result<String> {
    let container = format!("{SURREALDB_CONTAINER}-{instance}");
    let storage_dir = data_dir.join(format!("surrealdb_{instance}"));
    std::fs::create_dir_all(&storage_dir)?;
    let port = SURREALDB_PORT + hash_offset(instance);

    match inspect_container(&container) {
        ContainerState::Running => {}
        ContainerState::Stopped => { start_container(&container)?; }
        ContainerState::NotFound => {
            let status = Command::new("docker")
                .args([
                    "run", "-d", "--name", &container,
                    "-p", &format!("{port}:8000"),
                    "-v", &format!("{}:/data", storage_dir.canonicalize().unwrap_or(storage_dir).display()),
                    SURREALDB_IMAGE, "start", "--bind", "0.0.0.0:8000", "file:/data/synapse.db",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()?;
            if !status.success() {
                bail!("Failed to create SurrealDB container on port {port}");
            }
        }
    }

    wait_for_port(port, Duration::from_secs(30)).await?;
    Ok(format!("http://localhost:{port}"))
}

/// Compute a small port offset from an instance name to avoid collisions.
fn hash_offset(instance: &str) -> u16 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    if instance == "default" {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    instance.hash(&mut hasher);
    (hasher.finish() % 100) as u16
}

/// Poll a TCP port on localhost until it accepts connections.
async fn wait_for_port(port: u16, timeout: Duration) -> Result<()> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(500);

    loop {
        if start.elapsed() > timeout {
            bail!(
                "Timed out waiting for localhost:{port} to become ready after {}s",
                timeout.as_secs()
            );
        }

        let ready = tokio::task::spawn_blocking(move || {
            std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok()
        })
        .await
        .unwrap_or(false);

        if ready {
            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }
}
