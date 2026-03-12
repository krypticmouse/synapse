use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "synapse", version, about = "Terraform for Agent Memory")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new Synapse project
    Init,

    /// Validate a .mnm file (syntax + type check)
    Check {
        /// Path to the .mnm file
        #[arg(default_value = "synapse.mnm")]
        file: String,
    },

    /// Show an execution plan for a .mnm file
    Plan {
        /// Path to the .mnm file
        #[arg(default_value = "synapse.mnm")]
        file: String,
    },

    /// Compile and start the runtime
    Apply {
        /// Path to the .mnm file
        #[arg(default_value = "synapse.mnm")]
        file: String,

        /// Port to serve on (overrides config)
        #[arg(short, long)]
        port: Option<u16>,

        /// Run as a background daemon
        #[arg(short, long)]
        daemon: bool,
    },

    /// Show runtime status
    Status,

    /// Hot-reload the runtime
    Reload,

    /// View runtime logs
    Logs {
        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Log level filter
        #[arg(short, long)]
        level: Option<String>,
    },

    /// Inspect all databases: show tables, record counts, and contents
    Inspect {
        /// Filter by backend: sqlite, qdrant, neo4j
        #[arg(short, long)]
        backend: Option<String>,

        /// Filter by memory type name
        #[arg(short, long)]
        memory: Option<String>,

        /// Show only counts, not full records
        #[arg(short, long)]
        compact: bool,
    },

    /// Clear all records from all configured databases
    Clear,

    /// Stop and destroy the runtime
    Destroy {
        /// Also delete all persisted data
        #[arg(long)]
        purge: bool,
    },

    /// Execute a named query against the runtime
    Query {
        /// Query name
        name: String,

        /// Query params as JSON
        #[arg(default_value = "{}")]
        params: String,
    },

    /// Emit an event to trigger a handler
    Emit {
        /// Event name
        event: String,

        /// Event payload as JSON
        #[arg(default_value = "{}")]
        payload: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let cli = Cli::parse();

    let is_apply = matches!(cli.command, Command::Apply { .. });
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let console_layer = fmt::layer();

    let file_layer = if is_apply {
        let _ = std::fs::create_dir_all(".synapse");
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(".synapse/runtime.log")
            .ok()
            .map(|f| {
                fmt::layer()
                    .with_writer(std::sync::Mutex::new(f))
                    .with_ansi(false)
            })
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    match cli.command {
        Command::Init => commands::init::run(),
        Command::Check { file } => commands::check::run(&file),
        Command::Plan { file } => commands::plan::run(&file),
        Command::Apply { file, port, daemon } => commands::apply::run(&file, port, daemon).await,
        Command::Status => commands::status::run().await,
        Command::Inspect { backend, memory, compact } => {
            commands::inspect::run(backend.as_deref(), memory.as_deref(), compact).await
        }
        Command::Clear => commands::clear::run().await,
        Command::Reload => commands::reload::run().await,
        Command::Logs { follow, level } => commands::logs::run(follow, level.as_deref()),
        Command::Destroy { purge } => commands::destroy::run(purge),
        Command::Query { name, params } => commands::query::run(&name, &params).await,
        Command::Emit { event, payload } => commands::emit::run(&event, &payload).await,
    }
}
