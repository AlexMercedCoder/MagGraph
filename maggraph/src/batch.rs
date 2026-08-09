//! Transaction-like reviewed memory operations with rollback on failure.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{MagGraphError, Result};
use crate::index::GraphIndex;
use crate::node::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryBatchOperation {
    UpdateBody {
        id: String,
        body: String,
    },
    Suppress {
        id: String,
        reason: Option<String>,
    },
    Unsuppress {
        id: String,
    },
    Merge {
        target_id: String,
        source_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBatchResult {
    pub applied: Vec<String>,
}

pub fn apply_memory_batch(
    index: &mut GraphIndex,
    operations: &[MemoryBatchOperation],
) -> Result<MemoryBatchResult> {
    validate_memory_batch(index, operations)?;
    let snapshots = snapshots(index, operations)?;
    let mut applied = Vec::new();
    for operation in operations {
        let result = apply_one(index, operation);
        match result {
            Ok(label) => applied.push(label),
            Err(error) => {
                restore(index, &snapshots)?;
                return Err(error);
            }
        }
    }
    Ok(MemoryBatchResult { applied })
}

pub fn validate_memory_batch(
    index: &GraphIndex,
    operations: &[MemoryBatchOperation],
) -> Result<()> {
    let mut removed = BTreeSet::new();
    for operation in operations {
        match operation {
            MemoryBatchOperation::UpdateBody { id, .. }
            | MemoryBatchOperation::Suppress { id, .. }
            | MemoryBatchOperation::Unsuppress { id } => {
                require_available(index, id, &removed)?;
            }
            MemoryBatchOperation::Merge {
                target_id,
                source_id,
            } => {
                if target_id == source_id {
                    return Err(MagGraphError::Index(
                        "cannot merge a node into itself".to_string(),
                    ));
                }
                require_available(index, target_id, &removed)?;
                require_available(index, source_id, &removed)?;
                removed.insert(source_id.clone());
            }
        }
    }
    Ok(())
}

fn require_available(index: &GraphIndex, id: &str, removed: &BTreeSet<String>) -> Result<()> {
    if removed.contains(id) || !index.contains(id) {
        return Err(MagGraphError::NodeNotFound { id: id.to_string() });
    }
    Ok(())
}

fn snapshots(
    index: &GraphIndex,
    operations: &[MemoryBatchOperation],
) -> Result<BTreeMap<String, Node>> {
    let mut ids = BTreeSet::new();
    for operation in operations {
        match operation {
            MemoryBatchOperation::UpdateBody { id, .. }
            | MemoryBatchOperation::Suppress { id, .. }
            | MemoryBatchOperation::Unsuppress { id } => {
                ids.insert(id.clone());
            }
            MemoryBatchOperation::Merge {
                target_id,
                source_id,
            } => {
                ids.insert(target_id.clone());
                ids.insert(source_id.clone());
            }
        }
    }
    ids.into_iter()
        .map(|id| index.read_node(&id).map(|node| (id, node)))
        .collect()
}

fn apply_one(index: &mut GraphIndex, operation: &MemoryBatchOperation) -> Result<String> {
    match operation {
        MemoryBatchOperation::UpdateBody { id, body } => {
            let mut node = index.read_node(id)?;
            node.body = body.clone();
            index.update_node(node)?;
            Ok(format!("update:{id}"))
        }
        MemoryBatchOperation::Suppress { id, reason } => {
            index.suppress_node(id, reason.as_deref())?;
            Ok(format!("suppress:{id}"))
        }
        MemoryBatchOperation::Unsuppress { id } => {
            index.unsuppress_node(id)?;
            Ok(format!("unsuppress:{id}"))
        }
        MemoryBatchOperation::Merge {
            target_id,
            source_id,
        } => {
            index.merge_nodes(target_id, source_id)?;
            Ok(format!("merge:{source_id}->{target_id}"))
        }
    }
}

fn restore(index: &mut GraphIndex, snapshots: &BTreeMap<String, Node>) -> Result<()> {
    for node in snapshots.values() {
        node.write_to(index.root_path())?;
    }
    index.rescan()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn index() -> (TempDir, GraphIndex) {
        let temp = TempDir::new().expect("tempdir");
        for id in ["a", "b"] {
            fs::write(
                temp.path().join(format!("{id}.md")),
                format!("---\nid: {id}\ntype: decision\n---\nBody {id}.\n"),
            )
            .expect("write node");
        }
        let index = GraphIndex::open(temp.path()).expect("open");
        (temp, index)
    }

    #[test]
    fn batch_applies_reviewed_operations() {
        let (_temp, mut index) = index();
        let result = apply_memory_batch(
            &mut index,
            &[
                MemoryBatchOperation::UpdateBody {
                    id: "a".to_string(),
                    body: "Updated.".to_string(),
                },
                MemoryBatchOperation::Suppress {
                    id: "b".to_string(),
                    reason: Some("stale".to_string()),
                },
            ],
        )
        .expect("batch");

        assert_eq!(result.applied, ["update:a", "suppress:b"]);
        assert_eq!(index.read_node("a").unwrap().body, "Updated.\n");
        assert!(crate::query::is_suppressed(&index.read_node("b").unwrap()));
    }

    #[test]
    fn batch_prevalidation_prevents_partial_changes() {
        let (_temp, mut index) = index();
        let error = apply_memory_batch(
            &mut index,
            &[
                MemoryBatchOperation::UpdateBody {
                    id: "a".to_string(),
                    body: "Must not persist.".to_string(),
                },
                MemoryBatchOperation::Suppress {
                    id: "missing".to_string(),
                    reason: None,
                },
            ],
        )
        .unwrap_err();

        assert!(matches!(error, MagGraphError::NodeNotFound { .. }));
        assert_eq!(index.read_node("a").unwrap().body, "Body a.\n");
    }
}
