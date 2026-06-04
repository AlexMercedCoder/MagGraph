use clap::Args;
use maggraph::{ResolvedConfig, Result, SyncEngine};

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Skip creating the .maggraph metadata directory
    #[arg(long)]
    pub no_metadata_dir: bool,

    /// Initialize or attach Git repository for [sync]
    #[arg(long)]
    pub git: bool,
}

pub fn run(resolved: &ResolvedConfig, args: &InitArgs) -> Result<()> {
    resolved.initialize_graph_root(!args.no_metadata_dir)?;
    tracing::info!(root = %resolved.root_path.display(), "initialized graph root");

    if args.git || resolved.config.sync.is_some() {
        let sync = SyncEngine::init(resolved)?;
        tracing::info!(
            root = %sync.root_path().display(),
            role = ?sync.role(),
            "initialized git repository"
        );
    }

    Ok(())
}
