use clap::Args;
use maggraph::{GraphIndex, QueryOptions, ResolvedConfig, Result};

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Text to search across ids, types, links, frontmatter, and body
    #[arg(default_value = "")]
    pub query: String,

    /// Filter by node type
    #[arg(long)]
    pub node_type: Option<String>,

    /// Require a tag. Repeat for multiple required tags.
    #[arg(long = "tag")]
    pub tags: Vec<String>,

    /// Include nodes marked suppressed=true
    #[arg(long)]
    pub include_suppressed: bool,

    /// Maximum results
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// Only include nodes modified after this Unix timestamp
    #[arg(long)]
    pub modified_since_unix: Option<i64>,

    /// Output format: markdown or json
    #[arg(long, default_value = "markdown")]
    pub format: String,
}

pub fn run(resolved: &ResolvedConfig, args: &SearchArgs) -> Result<()> {
    let index = GraphIndex::open(&resolved.root_path)?;
    let results = index.search(&QueryOptions {
        text: if args.query.is_empty() {
            None
        } else {
            Some(args.query.clone())
        },
        node_type: args.node_type.clone(),
        tags: args.tags.clone(),
        include_suppressed: args.include_suppressed,
        limit: args.limit,
        modified_since_unix: args.modified_since_unix,
    })?;
    match args.format.as_str() {
        "markdown" => {
            println!("# MagGraph Search Results\n");
            println!("- **Query:** `{}`", args.query);
            println!("- **Results:** {}\n", results.len());
            for item in results {
                println!("## `{}` ({})", item.id, item.node_type);
                println!("- **Score:** {}", item.score);
                println!("- **File:** `{}`", item.relative_path);
                println!("- **Matched:** {}", item.matched.join(", "));
                if let Some(modified) = item.modified_unix {
                    println!("- **Modified:** {modified}");
                }
                if !item.summary.is_empty() {
                    println!("\n{}", item.summary);
                }
                println!();
            }
        }
        "json" => {
            let json = serde_json::to_string_pretty(
                &results
                    .iter()
                    .map(|item| {
                        serde_json::json!({
                            "id": item.id,
                            "type": item.node_type,
                            "relative_path": item.relative_path,
                            "score": item.score,
                            "matched": item.matched,
                            "summary": item.summary,
                            "modified_unix": item.modified_unix,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| maggraph::MagGraphError::Index(e.to_string()))?;
            println!("{json}");
        }
        other => {
            return Err(maggraph::MagGraphError::ConfigValidation(format!(
                "unsupported output format {other:?}; use markdown or json"
            )));
        }
    }
    Ok(())
}
