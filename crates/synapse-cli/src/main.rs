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
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init => commands::init::run(),
        Command::Check { file } => commands::check::run(&file),
        Command::Plan { file } => commands::plan::run(&file),
        Command::Apply { file, port, daemon } => commands::apply::run(&file, port, daemon).await,
        Command::Status => commands::status::run().await,
        Command::Reload => commands::reload::run(),
        Command::Logs { follow, level } => {
            commands::logs::run(follow, level.as_deref())
        }
        Command::Destroy { purge } => commands::destroy::run(purge),
        Command::Query { name, params } => commands::query::run(&name, &params).await,
        Command::Emit { event, payload } => commands::emit::run(&event, &payload).await,
    }
}
