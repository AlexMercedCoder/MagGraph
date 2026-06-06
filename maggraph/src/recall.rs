use std::collections::BTreeMap;

use serde_yaml::Value;

use crate::error::Result;
use crate::index::GraphIndex;
use crate::query::summarize_body;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallBundle {
    pub id: String,
    pub node_type: String,
    pub summary: String,
    pub body_excerpt: String,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    pub metadata: BTreeMap<String, Value>,
    pub relevance_reason: String,
}

impl RecallBundle {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("## `{}` ({})\n\n", self.id, self.node_type));
        if !self.relevance_reason.is_empty() {
            out.push_str(&format!("- **Reason:** {}\n", self.relevance_reason));
        }
        if !self.links.is_empty() {
            out.push_str(&format!("- **Links:** {}\n", self.links.join(", ")));
        }
        if !self.backlinks.is_empty() {
            out.push_str(&format!("- **Backlinks:** {}\n", self.backlinks.join(", ")));
        }
        out.push('\n');
        if !self.summary.is_empty() {
            out.push_str(&format!("**Summary:** {}\n\n", self.summary));
        }
        if !self.body_excerpt.is_empty() {
            out.push_str("```markdown\n");
            out.push_str(&self.body_excerpt);
            if !self.body_excerpt.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        out
    }
}

pub fn recall_bundle(
    index: &GraphIndex,
    id: &str,
    reason: impl Into<String>,
    body_chars: usize,
) -> Result<RecallBundle> {
    let node = index.read_node(id)?;
    let backlinks = index.backlinks(id)?;
    let body_excerpt = node
        .body
        .chars()
        .take(body_chars.max(1))
        .collect::<String>();
    Ok(RecallBundle {
        id: node.id().to_string(),
        node_type: node.metadata.node_type.clone(),
        summary: summarize_body(&node),
        body_excerpt,
        links: node.metadata.links.clone(),
        backlinks,
        metadata: node.metadata.extra.clone(),
        relevance_reason: reason.into(),
    })
}
