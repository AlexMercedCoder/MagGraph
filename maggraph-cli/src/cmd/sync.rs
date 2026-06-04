use clap::{Args, Subcommand};
use maggraph::{ResolvedConfig, Result, SyncEngine};

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub action: SyncAction,
}

#[derive(Debug, Subcommand)]
pub enum SyncAction {
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

pub fn run(resolved: &ResolvedConfig, args: &SyncArgs) -> Result<()> {
    match &args.action {
        SyncAction::Init => {
            let sync = SyncEngine::init(resolved)?;
            tracing::info!(
                root = %sync.root_path().display(),
                role = ?sync.role(),
                "sync repository ready"
            );
        }
        SyncAction::Status => {
            let sync = SyncEngine::open(resolved)?;
            let status = sync.status()?;
            println!("branch: {}", status.branch);
            println!("uncommitted: {}", status.uncommitted);
            println!("ahead: {}", status.ahead);
            println!("behind: {}", status.behind);
            println!("clean: {}", status.clean);
        }
        SyncAction::Pull => {
            let mut sync = open_or_clone(resolved)?;
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
            let mut sync = SyncEngine::open(resolved)?;
            let result = sync.commit_and_push(message)?;
            if let Some(commit) = result.commit {
                println!("pushed commit {commit}");
            } else {
                println!("nothing to commit; push complete");
            }
        }
    }
    Ok(())
}

fn open_or_clone(resolved: &ResolvedConfig) -> Result<SyncEngine> {
    SyncEngine::open(resolved).or_else(|err| {
        if matches!(err, maggraph::MagGraphError::Git { .. }) {
            SyncEngine::clone_follower(resolved)
        } else {
            Err(err)
        }
    })
}
