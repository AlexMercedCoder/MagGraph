mod frontmatter;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

pub use frontmatter::{parse_markdown_node, serialize_markdown_node};

use crate::error::{MagGraphError, Result};

/// Metadata stored in a node's YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    /// External asset URI (`source` or `source_uri` in frontmatter).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "source_uri")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// A markdown graph node: frontmatter metadata plus body content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub metadata: NodeMetadata,
    pub body: String,
    /// Path relative to the graph root (e.g. `welcome.md`).
    pub relative_path: PathBuf,
}

impl Node {
    /// Load a node from a markdown file under `root_path`.
    pub fn from_file(path: impl AsRef<Path>, root_path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let root_path = root_path.as_ref();

        let raw = fs::read_to_string(path).map_err(|source| MagGraphError::NodeRead {
            path: path.to_path_buf(),
            source,
        })?;

        let (metadata, body) =
            parse_markdown_node(&raw, path).map_err(|message| MagGraphError::NodeParse {
                path: path.to_path_buf(),
                message,
            })?;

        let relative_path = path
            .strip_prefix(root_path)
            .map_err(|_| MagGraphError::NodeParse {
                path: path.to_path_buf(),
                message: format!(
                    "node path {} is not under graph root {}",
                    path.display(),
                    root_path.display()
                ),
            })?;

        Ok(Self {
            metadata,
            body,
            relative_path: relative_path.to_path_buf(),
        })
    }

    /// Serialize this node to markdown with YAML frontmatter.
    pub fn to_markdown(&self) -> Result<String> {
        serialize_markdown_node(&self.metadata, &self.body).map_err(|message| {
            MagGraphError::NodeParse {
                path: self.relative_path.clone(),
                message,
            }
        })
    }

    /// Write this node to disk under `root_path`.
    pub fn write_to(&self, root_path: impl AsRef<Path>) -> Result<()> {
        let path = root_path.as_ref().join(&self.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| MagGraphError::NodeWrite {
                path: path.clone(),
                source,
            })?;
        }

        let contents = self.to_markdown()?;
        fs::write(&path, contents).map_err(|source| MagGraphError::NodeWrite { path, source })
    }

    pub fn id(&self) -> &str {
        &self.metadata.id
    }
}

/// Input for creating a new node on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNode {
    pub metadata: NodeMetadata,
    pub body: String,
    /// Target filename relative to graph root (e.g. `customer_churn_q2.md`).
    pub relative_path: PathBuf,
}

impl NewNode {
    pub fn into_node(self) -> Node {
        Node {
            metadata: self.metadata,
            body: self.body,
            relative_path: self.relative_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn parses_prd_example_schema() {
        let raw = r#"---
id: "customer_churn_q2"
type: "external_asset"
source: "s3://lake/churn_data.parquet"
importance: 8
links: ["retention_strategy_01"]
---
# Customer Churn Q2 Analysis
"#;

        let (metadata, body) = parse_markdown_node(raw, Path::new("test.md")).expect("parse");

        assert_eq!(metadata.id, "customer_churn_q2");
        assert_eq!(metadata.node_type, "external_asset");
        assert_eq!(
            metadata.source.as_deref(),
            Some("s3://lake/churn_data.parquet")
        );
        assert_eq!(metadata.links, vec!["retention_strategy_01"]);
        assert_eq!(
            metadata.extra.get("importance").and_then(Value::as_i64),
            Some(8)
        );
        assert_eq!(body.trim(), "# Customer Churn Q2 Analysis");
    }

    #[test]
    fn round_trip_preserves_unknown_frontmatter_keys() {
        let raw = r#"---
id: "note_a"
type: "note"
importance: 8
tags: ["alpha", "beta"]
custom_flag: true
---
Body text.
"#;

        let (metadata, body) = parse_markdown_node(raw, Path::new("note_a.md")).expect("parse");
        let serialized = serialize_markdown_node(&metadata, &body).expect("serialize");
        let (metadata2, body2) =
            parse_markdown_node(&serialized, Path::new("note_a.md")).expect("re-parse");

        assert_eq!(metadata, metadata2);
        assert_eq!(body, body2);
        assert!(serialized.contains("importance: 8"));
        assert!(serialized.contains("custom_flag: true"));
        assert!(serialized.contains("tags:"));
    }

    #[test]
    fn loads_node_from_file() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("create root");

        let path = root.join("welcome.md");
        fs::write(
            &path,
            r#"---
id: "welcome"
type: "note"
links: ["getting_started"]
---
# Welcome
"#,
        )
        .expect("write");

        let node = Node::from_file(&path, &root).expect("load");
        assert_eq!(node.id(), "welcome");
        assert_eq!(node.relative_path, PathBuf::from("welcome.md"));
    }

    #[test]
    fn write_to_creates_file() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("create root");

        let node = Node {
            metadata: NodeMetadata {
                id: "new_node".into(),
                node_type: "note".into(),
                source: None,
                links: vec![],
                extra: BTreeMap::new(),
            },
            body: "# Hello\n".into(),
            relative_path: PathBuf::from("subdir/new_node.md"),
        };

        node.write_to(&root).expect("write");
        assert!(root.join("subdir/new_node.md").is_file());

        let reloaded = Node::from_file(root.join("subdir/new_node.md"), &root).expect("reload");
        assert_eq!(reloaded, node);
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let err = parse_markdown_node("# No frontmatter", Path::new("x.md")).expect_err("error");
        assert!(err.contains("frontmatter"));
    }

    #[test]
    fn rejects_missing_required_id() {
        let raw = r#"---
type: "note"
---
Body
"#;
        let err = parse_markdown_node(raw, Path::new("x.md")).expect_err("error");
        assert!(err.contains("id"));
    }
}
