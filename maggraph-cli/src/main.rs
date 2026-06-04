use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use maggraph::{MagGraphConfig, Result, SyncEngine};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "maggraph",
    about = "In-process Git-backed graph database for AI"
)]
struct Cli {
    /// Path to maggraph.toml
    #[arg(long, global = true, default_value = "maggraph.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize the graph root (and optional Git repo when [sync] is configured)
    Init {
        /// Skip creating the .maggraph metadata directory
        #[arg(long)]
        no_metadata_dir: bool,

        /// Initialize or attach Git repository for [sync]
        #[arg(long)]
        git: bool,
    },
    /// Git sync operations (requires [sync] in maggraph.toml)
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
}

#[derive(Debug, Subcommand)]
enum SyncAction {
    /// Show working tree and upstream status
    Status,
    /// Fetch and merge from remote
    Pull,
    /// Commit outstanding changes and push (leader only)
    Push {
        /// Commit message when there are uncommitted changes
        #[arg(long, default_value = "MagGraph sync")]
        message: String,
    },
    /// Initialize Git repository and remote (leader or follower)
    Init,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let resolved = MagGraphConfig::load(&cli.config)?;

    match cli.command {
        Some(Commands::Init {
            no_metadata_dir,
            git,
        }) => {
            resolved.initialize_graph_root(!no_metadata_dir)?;
            tracing::info!(root = %resolved.root_path.display(), "initialized graph root");

            if git || resolved.config.sync.is_some() {
                let sync = SyncEngine::init(&resolved)?;
                tracing::info!(
                    root = %sync.root_path().display(),
                    role = ?sync.role(),
                    "initialized git repository"
                );
            }
        }
        Some(Commands::Sync { action }) => match action {
            SyncAction::Init => {
                let sync = SyncEngine::init(&resolved)?;
                tracing::info!(
                    root = %sync.root_path().display(),
                    role = ?sync.role(),
                    "sync repository ready"
                );
            }
            SyncAction::Status => {
                let sync = SyncEngine::open(&resolved)?;
                let status = sync.status()?;
                println!("branch: {}", status.branch);
                println!("uncommitted: {}", status.uncommitted);
                println!("ahead: {}", status.ahead);
                println!("behind: {}", status.behind);
                println!("clean: {}", status.clean);
            }
            SyncAction::Pull => {
                let mut sync = open_or_clone(&resolved)?;
                let result = sync.pull()?;
                if result.conflicts.is_empty() {
                    println!(
                        "pull complete (merged={}, fast_forward={})",
                        result.merged, result.fast_forward
                    );
                } else {
                    println!("pull paused with merge conflicts:");
                    for path in &result.conflicts {
                        println!("  {}", path.display());
                    }
                }
            }
            SyncAction::Push { message } => {
                let mut sync = SyncEngine::open(&resolved)?;
                let result = sync.commit_and_push(&message)?;
                if let Some(commit) = result.commit {
                    println!("pushed commit {commit}");
                } else {
                    println!("nothing to commit; push complete");
                }
            }
        },
        None => {
            tracing::info!(
                mode = ?resolved.config.storage.mode,
                root = %resolved.root_path.display(),
                "loaded configuration"
            );
        }
    }

    Ok(())
}

fn open_or_clone(resolved: &maggraph::ResolvedConfig) -> Result<SyncEngine> {
    SyncEngine::open(resolved).or_else(|err| {
        if matches!(err, maggraph::MagGraphError::Git { .. }) {
            SyncEngine::clone_follower(resolved)
        } else {
            Err(err)
        }
    })
}
