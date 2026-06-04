use clap::Args;
use maggraph::{
    agent::{GraphSchema, SkillRenderContext},
    write_skill_md, GraphIndex, ResolvedConfig, Result, StorageMode, SyncEngine,
};

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Skip creating the .maggraph metadata directory
    #[arg(long)]
    pub no_metadata_dir: bool,

    /// Initialize or attach Git repository for [sync]
    #[arg(long)]
    pub git: bool,

    /// Generate SKILL.md in the graph root after initialization
    #[arg(long)]
    pub skill: bool,
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

    if args.skill {
        write_skill_for_graph(resolved)?;
    }

    Ok(())
}

fn write_skill_for_graph(resolved: &ResolvedConfig) -> Result<()> {
    let index = GraphIndex::open(&resolved.root_path)?;
    let schema = GraphSchema::introspect(&index)?;
    let ctx = SkillRenderContext {
        graph_root: &resolved.root_path,
        config_path: Some(&resolved.config_path),
        storage_mode: Some(match resolved.config.storage.mode {
            StorageMode::Local => "local",
            StorageMode::Lakehouse => "lakehouse",
        }),
        maggraph_version: env!("CARGO_PKG_VERSION"),
    };
    let skill_path = resolved.root_path.join("SKILL.md");
    write_skill_md(&skill_path, &schema, &ctx)?;
    tracing::info!(path = %skill_path.display(), "wrote SKILL.md");
    println!("SKILL.md: {}", skill_path.display());
    Ok(())
}
