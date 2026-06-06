use clap::Args;
use maggraph::{GraphIndex, ResolvedConfig, Result};

#[derive(Debug, Args)]
pub struct RecallArgs {
    /// Node id to recall
    pub node_id: String,

    /// Why this node is being recalled
    #[arg(long, default_value = "requested")]
    pub reason: String,

    /// Maximum body excerpt characters
    #[arg(long, default_value_t = 1200)]
    pub body_chars: usize,

    /// Output format: markdown or json
    #[arg(long, default_value = "markdown")]
    pub format: String,
}

pub fn run(resolved: &ResolvedConfig, args: &RecallArgs) -> Result<()> {
    let index = GraphIndex::open(&resolved.root_path)?;
    let bundle = index.recall_bundle(&args.node_id, &args.reason, args.body_chars)?;
    match args.format.as_str() {
        "markdown" => print!("{}", bundle.to_markdown()),
        "json" => {
            let json = serde_json::to_string_pretty(&serde_json::json!({
                "id": bundle.id,
                "type": bundle.node_type,
                "summary": bundle.summary,
                "body_excerpt": bundle.body_excerpt,
                "links": bundle.links,
                "backlinks": bundle.backlinks,
                "relevance_reason": bundle.relevance_reason,
            }))
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
