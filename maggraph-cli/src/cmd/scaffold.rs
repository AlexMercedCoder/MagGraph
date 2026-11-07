use std::path::PathBuf;

use clap::Args;
use maggraph::{
    agent::{GraphSchema, McpScaffoldContext, ScaffoldOptions, SkillRenderContext},
    write_mcp_scaffold_with, write_skill_md_with, GraphIndex, ResolvedConfig, Result,
    SKILL_DIR_NAME,
};

#[derive(Debug, Args)]
pub struct ScaffoldArgs {
    /// Emit a FastMCP Python server scaffold
    #[arg(long)]
    pub mcp: bool,

    /// Output directory for generated artifacts (default: current directory)
    #[arg(long, default_value = ".")]
    pub output: PathBuf,

    /// Also write SKILL.md under the output directory (maggraph-skill/SKILL.md)
    #[arg(long)]
    pub skill: bool,

    /// Generate a read-only MCP server (omit create_node, update_node, delete_node)
    #[arg(long)]
    pub read_only: bool,
}

#[tracing::instrument(skip_all, fields(mcp = args.mcp, skill = args.skill, read_only = args.read_only))]
pub fn run(resolved: &ResolvedConfig, args: &ScaffoldArgs) -> Result<()> {
    if !args.mcp && !args.skill {
        return Err(maggraph::MagGraphError::ConfigValidation(
            "scaffold requires at least one of --mcp or --skill".into(),
        ));
    }

    let index = GraphIndex::open(&resolved.root_path)?;
    for skipped in index.skipped_files() {
        eprintln!(
            "note: skipped non-node file {} ({})",
            skipped.relative_path.display(),
            skipped.reason
        );
    }
    let schema = GraphSchema::introspect(&index)?;
    let options = if args.read_only {
        ScaffoldOptions::read_only()
    } else {
        ScaffoldOptions::default()
    };

    let skill_ctx = SkillRenderContext {
        graph_root: &resolved.root_path,
        config_path: Some(&resolved.config_path),
        storage_mode: Some(storage_mode_label(resolved)),
        maggraph_version: env!("CARGO_PKG_VERSION"),
    };

    if args.mcp {
        let mcp_dir = args.output.join("mcp_server");
        let mcp_ctx = McpScaffoldContext {
            graph_root: &resolved.root_path,
            config_path: &resolved.config_path,
            schema: &schema,
        };
        write_mcp_scaffold_with(&mcp_dir, &mcp_ctx, options)?;
        tracing::info!(path = %mcp_dir.display(), "wrote MCP server scaffold");
        println!("MCP scaffold: {}", mcp_dir.display());
    }

    if args.skill {
        // Keep generated docs out of the graph root: a SKILL.md there is not a node.
        let skill_path = args.output.join(SKILL_DIR_NAME).join("SKILL.md");
        write_skill_md_with(&skill_path, &schema, &skill_ctx, options)?;
        tracing::info!(path = %skill_path.display(), "wrote SKILL.md");
        println!("SKILL.md: {}", skill_path.display());
    }

    Ok(())
}

fn storage_mode_label(resolved: &ResolvedConfig) -> &'static str {
    match resolved.config.storage.mode {
        maggraph::StorageMode::Local => "local",
        maggraph::StorageMode::Lakehouse => "lakehouse",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn schema_introspect_basic_example() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/basic/knowledge_graph");
        let index = GraphIndex::open(&root).expect("open");
        let schema = GraphSchema::introspect(&index).expect("introspect");

        assert!(schema.node_count >= 2);
        assert!(schema.node_types.contains(&"note".to_string()));
        assert!(schema.node_ids.contains(&"welcome".to_string()));
        assert!(schema.edge_count >= 1);
    }
}
