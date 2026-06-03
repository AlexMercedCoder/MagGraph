use std::path::PathBuf;

use clap::Parser;
use maggraph::{MagGraphConfig, Result};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "maggraph",
    about = "In-process Git-backed graph database for AI"
)]
struct Cli {
    /// Path to maggraph.toml
    #[arg(long, default_value = "maggraph.toml")]
    config: PathBuf,

    /// Initialize the graph root (and .maggraph metadata dir) after loading config
    #[arg(long)]
    init: bool,

    /// Skip creating the .maggraph metadata directory when using --init
    #[arg(long)]
    no_metadata_dir: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let resolved = MagGraphConfig::load(&cli.config)?;

    if cli.init {
        resolved.initialize_graph_root(!cli.no_metadata_dir)?;
        tracing::info!(root = %resolved.root_path.display(), "initialized graph root");
    }

    tracing::info!(
        mode = ?resolved.config.storage.mode,
        root = %resolved.root_path.display(),
        "loaded configuration"
    );

    Ok(())
}
