use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{MagGraphError, Result};
use crate::node::{NewNode, Node, NodeMetadata};

/// Lightweight index entry for a graph node (metadata + path, no body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIndexEntry {
    pub metadata: NodeMetadata,
    pub relative_path: PathBuf,
}

impl NodeIndexEntry {
    pub fn id(&self) -> &str {
        &self.metadata.id
    }
}

/// In-memory index of markdown nodes under a graph root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphIndex {
    root_path: PathBuf,
    by_id: HashMap<String, NodeIndexEntry>,
    by_path: HashMap<PathBuf, String>,
}

impl GraphIndex {
    /// Open an index at `root_path`, performing a full scan of `*.md` files.
    pub fn open(root_path: impl AsRef<Path>) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        if !root_path.is_dir() {
            return Err(MagGraphError::Index(format!(
                "graph root {} does not exist or is not a directory",
                root_path.display()
            )));
        }

        let mut index = Self {
            root_path,
            by_id: HashMap::new(),
            by_path: HashMap::new(),
        };
        index.rescan()?;
        Ok(index)
    }

    /// Full rescan of all markdown files under the graph root.
    pub fn rescan(&mut self) -> Result<()> {
        self.by_id.clear();
        self.by_path.clear();

        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }

            let relative_path = path.strip_prefix(&self.root_path).map_err(|_| {
                MagGraphError::Index(format!(
                    "failed to relativize path {} under {}",
                    path.display(),
                    self.root_path.display()
                ))
            })?;

            // Skip metadata directory if present inside graph root.
            if relative_path
                .components()
                .any(|component| component.as_os_str() == crate::config::METADATA_DIR_NAME)
            {
                continue;
            }

            let node = Node::from_file(path, &self.root_path)?;
            self.insert_entry(node.metadata, relative_path.to_path_buf())?;
        }

        Ok(())
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn get(&self, id: &str) -> Option<&NodeIndexEntry> {
        self.by_id.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &NodeIndexEntry)> {
        self.by_id.iter().map(|(id, entry)| (id.as_str(), entry))
    }

    /// Read the full node (including body) by id.
    pub fn read_node(&self, id: &str) -> Result<Node> {
        let entry = self
            .by_id
            .get(id)
            .ok_or_else(|| MagGraphError::NodeNotFound { id: id.to_string() })?;

        Node::from_file(self.root_path.join(&entry.relative_path), &self.root_path)
    }

    /// Create a new node on disk and update the index.
    pub fn create_node(&mut self, new_node: NewNode) -> Result<Node> {
        let node = new_node.into_node();
        if self.by_id.contains_key(node.id()) {
            return Err(MagGraphError::NodeAlreadyExists {
                id: node.id().to_string(),
                path: node.relative_path.clone(),
            });
        }

        if self.by_path.contains_key(&node.relative_path) {
            return Err(MagGraphError::Index(format!(
                "a node already exists at path {}",
                node.relative_path.display()
            )));
        }

        let target = self.root_path.join(&node.relative_path);
        if target.exists() {
            return Err(MagGraphError::NodeAlreadyExists {
                id: node.id().to_string(),
                path: node.relative_path.clone(),
            });
        }

        node.write_to(&self.root_path)?;
        self.insert_entry(node.metadata.clone(), node.relative_path.clone())?;
        Ok(node)
    }

    /// Update an existing node on disk and refresh the index.
    pub fn update_node(&mut self, node: Node) -> Result<()> {
        let existing = self
            .by_id
            .get(node.id())
            .ok_or_else(|| MagGraphError::NodeNotFound {
                id: node.id().to_string(),
            })?;

        if existing.relative_path != node.relative_path {
            return Err(MagGraphError::Index(format!(
                "cannot change node path for id {} from {} to {}",
                node.id(),
                existing.relative_path.display(),
                node.relative_path.display()
            )));
        }

        node.write_to(&self.root_path)?;
        self.by_id.insert(
            node.id().to_string(),
            NodeIndexEntry {
                metadata: node.metadata,
                relative_path: node.relative_path,
            },
        );
        Ok(())
    }

    /// Delete a node from disk and remove it from the index.
    pub fn delete_node(&mut self, id: &str) -> Result<()> {
        let entry = self
            .by_id
            .remove(id)
            .ok_or_else(|| MagGraphError::NodeNotFound { id: id.to_string() })?;

        self.by_path.remove(&entry.relative_path);

        let path = self.root_path.join(&entry.relative_path);
        if path.exists() {
            fs::remove_file(&path).map_err(|source| MagGraphError::NodeDelete { path, source })?;
        }

        Ok(())
    }

    fn insert_entry(&mut self, metadata: NodeMetadata, relative_path: PathBuf) -> Result<()> {
        let id = metadata.id.clone();

        if let Some(existing) = self.by_id.get(&id) {
            return Err(MagGraphError::DuplicateNodeId {
                id,
                first: existing.relative_path.clone(),
                second: relative_path,
            });
        }

        if let Some(existing_id) = self.by_path.get(&relative_path) {
            return Err(MagGraphError::Index(format!(
                "path {} is already indexed as node `{existing_id}`",
                relative_path.display()
            )));
        }

        self.by_id.insert(
            id.clone(),
            NodeIndexEntry {
                metadata,
                relative_path: relative_path.clone(),
            },
        );
        self.by_path.insert(relative_path, id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;

    use serde_yaml::Value;
    use tempfile::TempDir;

    fn write_example_graph(root: &Path) {
        fs::create_dir_all(root).expect("create root");
        fs::write(
            root.join("welcome.md"),
            r#"---
id: "welcome"
type: "note"
links: ["getting_started"]
---
# Welcome
"#,
        )
        .expect("write welcome");
        fs::write(
            root.join("getting_started.md"),
            r#"---
id: "getting_started"
type: "note"
links: ["welcome"]
---
# Getting Started
"#,
        )
        .expect("write getting_started");
    }

    #[test]
    fn scans_example_graph() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        write_example_graph(&root);

        let index = GraphIndex::open(&root).expect("open index");
        assert_eq!(index.len(), 2);
        assert!(index.contains("welcome"));
        assert!(index.contains("getting_started"));
        assert_eq!(
            index.get("welcome").unwrap().metadata.links,
            vec!["getting_started"]
        );
    }

    #[test]
    fn scans_basic_example_fixture() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir.join("../examples/basic/knowledge_graph");

        let index = GraphIndex::open(&root).expect("open example graph");
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn crud_round_trip() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("create root");

        let mut index = GraphIndex::open(&root).expect("open empty index");
        assert!(index.is_empty());

        let mut extra = BTreeMap::new();
        extra.insert("importance".into(), Value::Number(8.into()));

        let created = index
            .create_node(NewNode {
                metadata: NodeMetadata {
                    id: "customer_churn_q2".into(),
                    node_type: "external_asset".into(),
                    source: Some("s3://lake/churn_data.parquet".into()),
                    links: vec!["retention_strategy_01".into()],
                    extra,
                },
                body: "# Customer Churn Q2 Analysis\n".into(),
                relative_path: PathBuf::from("customer_churn_q2.md"),
            })
            .expect("create");

        assert_eq!(index.len(), 1);
        assert!(root.join("customer_churn_q2.md").is_file());

        let loaded = index.read_node("customer_churn_q2").expect("read");
        assert_eq!(loaded.metadata, created.metadata);
        assert_eq!(loaded.body, created.body);

        let updated = Node {
            metadata: NodeMetadata {
                links: vec!["retention_strategy_01".into(), "welcome".into()],
                ..loaded.metadata.clone()
            },
            body: "# Customer Churn Q2 Analysis\n\nUpdated.\n".into(),
            relative_path: loaded.relative_path.clone(),
        };
        index.update_node(updated.clone()).expect("update");

        let reloaded = index.read_node("customer_churn_q2").expect("re-read");
        assert_eq!(reloaded, updated);

        index.delete_node("customer_churn_q2").expect("delete");
        assert!(index.is_empty());
        assert!(!root.join("customer_churn_q2.md").exists());
    }

    #[test]
    fn detects_duplicate_ids_on_scan() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("create root");

        fs::write(
            root.join("a.md"),
            r#"---
id: "dup"
type: "note"
---
A
"#,
        )
        .expect("write a");
        fs::write(
            root.join("b.md"),
            r#"---
id: "dup"
type: "note"
---
B
"#,
        )
        .expect("write b");

        let err = GraphIndex::open(&root).expect_err("duplicate");
        assert!(matches!(err, MagGraphError::DuplicateNodeId { .. }));
    }

    #[test]
    fn create_rejects_existing_id() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        write_example_graph(&root);

        let mut index = GraphIndex::open(&root).expect("open");
        let err = index
            .create_node(NewNode {
                metadata: NodeMetadata {
                    id: "welcome".into(),
                    node_type: "note".into(),
                    source: None,
                    links: vec![],
                    extra: BTreeMap::new(),
                },
                body: "x".into(),
                relative_path: PathBuf::from("other.md"),
            })
            .expect_err("duplicate id");

        assert!(matches!(err, MagGraphError::NodeAlreadyExists { .. }));
    }
}
