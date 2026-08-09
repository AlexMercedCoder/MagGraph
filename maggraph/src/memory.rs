use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_yaml::Value;

use crate::node::{NewNode, NodeMetadata};

pub const MEMORY_TYPES: &[&str] = &[
    "preference",
    "project_fact",
    "decision",
    "task",
    "session_summary",
    "bookmark",
    "tool_failure",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryNodeKind {
    Preference,
    ProjectFact,
    Decision,
    Task,
    SessionSummary,
    Bookmark,
    ToolFailure,
}

/// Optional provenance, scope, and temporal metadata for an agent memory.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MemoryContext {
    pub project: Option<String>,
    pub source_task: Option<String>,
    pub source_session: Option<String>,
    pub source_tool: Option<String>,
    pub extraction_method: Option<String>,
    pub confidence: Option<f64>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub supersedes: Option<String>,
    pub canonical_id: Option<String>,
}

impl MemoryNodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::ProjectFact => "project_fact",
            Self::Decision => "decision",
            Self::Task => "task",
            Self::SessionSummary => "session_summary",
            Self::Bookmark => "bookmark",
            Self::ToolFailure => "tool_failure",
        }
    }
}

pub fn validate_memory_type(node_type: &str) -> bool {
    MEMORY_TYPES.contains(&node_type)
}

pub fn new_memory_node(
    id: impl Into<String>,
    kind: MemoryNodeKind,
    body: impl Into<String>,
    links: Vec<String>,
    extra: BTreeMap<String, Value>,
) -> NewNode {
    let id = id.into();
    NewNode {
        metadata: NodeMetadata {
            id: id.clone(),
            node_type: kind.as_str().to_string(),
            source: None,
            links,
            extra,
        },
        body: body.into(),
        relative_path: PathBuf::from(format!("{id}.md")),
    }
}

/// Create a memory node with normalized provenance and temporal frontmatter.
pub fn new_memory_node_with_context(
    id: impl Into<String>,
    kind: MemoryNodeKind,
    body: impl Into<String>,
    links: Vec<String>,
    context: MemoryContext,
) -> NewNode {
    let mut extra = BTreeMap::new();
    insert_string(&mut extra, "project", context.project);
    insert_string(&mut extra, "source_task", context.source_task);
    insert_string(&mut extra, "source_session", context.source_session);
    insert_string(&mut extra, "source_tool", context.source_tool);
    insert_string(&mut extra, "extraction_method", context.extraction_method);
    if let Some(confidence) = context.confidence {
        if let Ok(value) = serde_yaml::to_value(confidence.clamp(0.0, 1.0)) {
            extra.insert("confidence".to_string(), value);
        }
    }
    insert_string(&mut extra, "valid_from", context.valid_from);
    insert_string(&mut extra, "valid_until", context.valid_until);
    insert_string(&mut extra, "supersedes", context.supersedes);
    insert_string(&mut extra, "canonical_id", context.canonical_id);
    new_memory_node(id, kind, body, links, extra)
}

fn insert_string(extra: &mut BTreeMap<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value.filter(|item| !item.trim().is_empty()) {
        extra.insert(key.to_string(), Value::String(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contextual_memory_normalizes_provenance_and_confidence() {
        let node = new_memory_node_with_context(
            "decision",
            MemoryNodeKind::Decision,
            "Use hybrid retrieval.",
            Vec::new(),
            MemoryContext {
                project: Some("demo".to_string()),
                source_task: Some("task_1".to_string()),
                confidence: Some(1.5),
                canonical_id: Some("retrieval-decision".to_string()),
                ..MemoryContext::default()
            },
        );

        assert_eq!(node.metadata.extra["project"].as_str(), Some("demo"));
        assert_eq!(node.metadata.extra["source_task"].as_str(), Some("task_1"));
        assert_eq!(node.metadata.extra["confidence"].as_f64(), Some(1.0));
        assert_eq!(
            node.metadata.extra["canonical_id"].as_str(),
            Some("retrieval-decision")
        );
    }
}
