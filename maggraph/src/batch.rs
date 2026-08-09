//! Transaction-like reviewed memory operations with rollback on failure.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

const JOURNAL_VERSION: u8 = 1;
const TRANSACTIONS_DIR: &str = "transactions";

#[derive(Debug, Serialize, Deserialize)]
struct BatchJournal {
    version: u8,
    snapshots: Vec<JournalSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JournalSnapshot {
    relative_path: PathBuf,
    backup: PathBuf,
}

pub fn apply_memory_batch(
    index: &mut GraphIndex,
    operations: &[MemoryBatchOperation],
) -> Result<MemoryBatchResult> {
    validate_memory_batch(index, operations)?;
    let snapshots = snapshots(index, operations)?;
    let journal = prepare_journal(index.root_path(), &snapshots)?;
    let mut applied = Vec::new();
    for operation in operations {
        let result = apply_one(index, operation);
        match result {
            Ok(label) => applied.push(label),
            Err(error) => {
                restore_journal(index.root_path(), &journal)?;
                index.rescan()?;
                return Err(error);
            }
        }
    }
    fs::remove_dir_all(&journal).map_err(|error| {
        MagGraphError::Index(format!("failed to remove completed batch journal: {error}"))
    })?;
    Ok(MemoryBatchResult { applied })
}

/// Restore any prepared batch that did not reach journal cleanup before shutdown.
pub(crate) fn recover_incomplete_batches(root: &Path) -> Result<usize> {
    let transactions = transaction_root(root);
    if !transactions.exists() {
        return Ok(0);
    }
    let mut recovered = 0;
    for entry in fs::read_dir(&transactions).map_err(|error| {
        MagGraphError::Index(format!("failed to inspect batch journals: {error}"))
    })? {
        let path = entry
            .map_err(|error| {
                MagGraphError::Index(format!("failed to read batch journal: {error}"))
            })?
            .path();
        if !path.is_dir() {
            continue;
        }
        if path.join("manifest.toml").exists() {
            restore_journal(root, &path)?;
            recovered += 1;
        } else {
            fs::remove_dir_all(&path).map_err(|error| {
                MagGraphError::Index(format!(
                    "failed to remove unprepared batch journal: {error}"
                ))
            })?;
        }
    }
    Ok(recovered)
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

fn transaction_root(root: &Path) -> PathBuf {
    root.join(crate::config::METADATA_DIR_NAME)
        .join(TRANSACTIONS_DIR)
}

fn prepare_journal(root: &Path, snapshots: &BTreeMap<String, Node>) -> Result<PathBuf> {
    let journal_dir = transaction_root(root).join(Uuid::new_v4().to_string());
    let backup_dir = journal_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|error| {
        MagGraphError::Index(format!("failed to create batch journal: {error}"))
    })?;
    let mut journal = BatchJournal {
        version: JOURNAL_VERSION,
        snapshots: Vec::new(),
    };
    for (index, node) in snapshots.values().enumerate() {
        crate::security::validate_relative_node_path(&node.relative_path)?;
        let backup = PathBuf::from("backups").join(format!("{index:04}.md"));
        sync_write(&journal_dir.join(&backup), node.to_markdown()?.as_bytes())?;
        journal.snapshots.push(JournalSnapshot {
            relative_path: node.relative_path.clone(),
            backup,
        });
    }
    let manifest = toml::to_string(&journal).map_err(|error| {
        MagGraphError::Index(format!("failed to serialize batch journal: {error}"))
    })?;
    sync_write(&journal_dir.join("manifest.toml"), manifest.as_bytes())?;
    Ok(journal_dir)
}

fn restore_journal(root: &Path, journal_dir: &Path) -> Result<()> {
    let manifest_path = journal_dir.join("manifest.toml");
    let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
        MagGraphError::Index(format!(
            "failed to read batch journal {}: {error}",
            manifest_path.display()
        ))
    })?;
    let journal: BatchJournal = toml::from_str(&manifest)
        .map_err(|error| MagGraphError::Index(format!("failed to parse batch journal: {error}")))?;
    if journal.version != JOURNAL_VERSION {
        return Err(MagGraphError::Index(format!(
            "unsupported batch journal version {}",
            journal.version
        )));
    }
    for snapshot in journal.snapshots {
        crate::security::validate_relative_node_path(&snapshot.relative_path)?;
        crate::security::validate_relative_node_path(&snapshot.backup)?;
        let contents = fs::read_to_string(journal_dir.join(&snapshot.backup)).map_err(|error| {
            MagGraphError::Index(format!("failed to read batch backup: {error}"))
        })?;
        let (metadata, body) = crate::node::parse_markdown_node(&contents, &snapshot.relative_path)
            .map_err(|message| MagGraphError::NodeParse {
                path: snapshot.relative_path.clone(),
                message,
            })?;
        Node {
            metadata,
            body,
            relative_path: snapshot.relative_path,
        }
        .write_to(root)?;
    }
    fs::remove_dir_all(journal_dir).map_err(|error| {
        MagGraphError::Index(format!("failed to clear recovered batch journal: {error}"))
    })
}

fn sync_write(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file = File::create(path).map_err(|error| {
        MagGraphError::Index(format!("failed to create {}: {error}", path.display()))
    })?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            MagGraphError::Index(format!("failed to sync {}: {error}", path.display()))
        })
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

    #[test]
    fn reopen_recovers_a_partially_applied_batch() {
        let (temp, index) = index();
        let snapshots = snapshots(
            &index,
            &[MemoryBatchOperation::Merge {
                target_id: "a".to_string(),
                source_id: "b".to_string(),
            }],
        )
        .expect("snapshots");
        let _journal = prepare_journal(temp.path(), &snapshots).expect("journal");

        fs::write(
            temp.path().join("a.md"),
            "---\nid: a\ntype: decision\n---\nPartially updated.\n",
        )
        .expect("partial target");
        fs::remove_file(temp.path().join("b.md")).expect("partial delete");

        let reopened = GraphIndex::open(temp.path()).expect("recover and reopen");
        assert_eq!(reopened.read_node("a").unwrap().body, "Body a.\n");
        assert_eq!(reopened.read_node("b").unwrap().body, "Body b.\n");
        assert!(
            !transaction_root(temp.path()).exists()
                || transaction_root(temp.path())
                    .read_dir()
                    .unwrap()
                    .next()
                    .is_none()
        );
    }
}
