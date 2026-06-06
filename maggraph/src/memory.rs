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
