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

    let url = format!("http://localhost:{QDRANT_PORT}");
    wait_for_port(QDRANT_PORT, Duration::from_secs(30)).await?;
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
