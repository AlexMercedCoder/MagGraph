use clap::Args;
use maggraph::{traverse, GraphIndex, ResolvedConfig, Result, TraversalOrder};

#[derive(Debug, Args)]
pub struct QueryArgs {
    /// Start node id for traversal
    #[arg(long)]
    pub from: String,

    /// Maximum traversal depth (start node is depth 0)
    #[arg(long, default_value_t = 2)]
    pub depth: u32,

    /// Traversal order: bfs or dfs
    #[arg(long, value_parser = parse_order, default_value = "bfs")]
    pub order: TraversalOrder,

    /// Output format (only markdown is supported)
    #[arg(long, default_value = "markdown")]
    pub format: String,
}

fn parse_order(s: &str) -> std::result::Result<TraversalOrder, String> {
    match s.to_ascii_lowercase().as_str() {
        "bfs" => Ok(TraversalOrder::Bfs),
        "dfs" => Ok(TraversalOrder::Dfs),
        other => Err(format!("unknown order {other:?}; use bfs or dfs")),
    }
}

pub fn run(resolved: &ResolvedConfig, args: &QueryArgs) -> Result<()> {
    if args.format != "markdown" {
        return Err(maggraph::MagGraphError::ConfigValidation(format!(
            "unsupported output format {:?}; only markdown is supported",
            args.format
        )));
    }

    let index = GraphIndex::open(&resolved.root_path)?;
    let adjacency = index.adjacency()?;
    let result = traverse(&adjacency, &index, &args.from, args.depth, args.order)?;

    print!("{}", result.to_markdown(&index));
    Ok(())
}
