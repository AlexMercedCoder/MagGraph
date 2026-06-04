use std::path::Path;

use crate::config::{ResolvedConfig, SyncRole};
use crate::error::{MagGraphError, Result};
use crate::sync::lock::WriteLockGuard;
use crate::sync::policy::WritePolicy;
use crate::sync::repo::{GitRepository, PullResult, PushResult, SyncStatus, DEFAULT_REMOTE};

/// Git sync engine for leader/follower replication.
pub struct SyncEngine {
    git: GitRepository,
    role: SyncRole,
    remote_url: String,
}

impl SyncEngine {
    /// Open an existing Git repository at the configured graph root.
    pub fn open(resolved: &ResolvedConfig) -> Result<Self> {
        let sync = resolved
            .config
            .sync
            .as_ref()
            .ok_or(MagGraphError::SyncNotConfigured)?;

        if !resolved.root_path.is_dir() {
            return Err(MagGraphError::Git {
                message: format!(
                    "graph root {} does not exist; run with --init first",
                    resolved.root_path.display()
                ),
            });
        }

        let git = GitRepository::open(&resolved.root_path)?;

        Ok(Self {
            git,
            role: sync.role.clone(),
            remote_url: sync.remote_url.clone(),
        })
    }

    /// Initialize (or attach to) a Git repository and configure the remote.
    pub fn init(resolved: &ResolvedConfig) -> Result<Self> {
        let sync = resolved
            .config
            .sync
            .as_ref()
            .ok_or(MagGraphError::SyncNotConfigured)?;

        let mut git = GitRepository::init(&resolved.root_path)?;
        git.ensure_remote(DEFAULT_REMOTE, &sync.remote_url)?;
        git.ensure_initial_commit()?;

        Ok(Self {
            git,
            role: sync.role.clone(),
            remote_url: sync.remote_url.clone(),
        })
    }

    /// Clone a follower graph from `remote_url` into `root_path`.
    pub fn clone_follower(resolved: &ResolvedConfig) -> Result<Self> {
        let sync = resolved
            .config
            .sync
            .as_ref()
            .ok_or(MagGraphError::SyncNotConfigured)?;

        if sync.role != SyncRole::Follower {
            return Err(MagGraphError::ConfigValidation(
                "clone_follower requires [sync].role = \"follower\"".into(),
            ));
        }

        let git = if resolved.root_path.join(".git").exists() {
            GitRepository::open(&resolved.root_path)?
        } else if resolved.root_path.exists() {
            return Err(MagGraphError::Git {
                message: format!(
                    "graph root {} exists but is not a git clone; remove it or pick an empty path",
                    resolved.root_path.display()
                ),
            });
        } else {
            GitRepository::clone_from(&sync.remote_url, &resolved.root_path)?
        };

        Ok(Self {
            git,
            role: sync.role.clone(),
            remote_url: sync.remote_url.clone(),
        })
    }

    pub fn role(&self) -> &SyncRole {
        &self.role
    }

    pub fn root_path(&self) -> &Path {
        self.git.root_path()
    }

    pub fn remote_url(&self) -> &str {
        &self.remote_url
    }

    pub fn write_policy(&self) -> WritePolicy<'static> {
        match self.role {
            SyncRole::Follower => WritePolicy::follower(),
            SyncRole::Leader => WritePolicy::from_role(SyncRole::Leader),
        }
    }

    pub fn status(&self) -> Result<SyncStatus> {
        self.git.status()
    }

    pub fn pull(&mut self) -> Result<PullResult> {
        self.git.pull(DEFAULT_REMOTE)
    }

    pub fn push(&mut self) -> Result<PushResult> {
        if self.role != SyncRole::Leader {
            return Err(MagGraphError::ReadOnlyRole {
                role: "follower".into(),
            });
        }

        self.git.push(DEFAULT_REMOTE)
    }

    pub fn commit_all(&mut self, message: &str) -> Result<Option<String>> {
        if self.role != SyncRole::Leader {
            return Err(MagGraphError::ReadOnlyRole {
                role: "follower".into(),
            });
        }

        self.git.commit_all(message)
    }

    /// Commit outstanding changes and push to the configured remote (leader only).
    pub fn commit_and_push(&mut self, message: &str) -> Result<PushResult> {
        self.commit_all(message)?;
        self.push()
    }

    /// Acquire the leader write lock and run `f` with an authorized policy.
    pub fn with_leader_write<F, T>(&self, holder: &str, f: F) -> Result<T>
    where
        F: FnOnce(WritePolicy<'_>) -> Result<T>,
    {
        if self.role != SyncRole::Leader {
            return Err(MagGraphError::ReadOnlyRole {
                role: role_label(&self.role).to_string(),
            });
        }

        let lock = WriteLockGuard::acquire(self.git.root_path(), holder)?;
        f(WritePolicy::leader_with_lock(&lock))
    }
}

fn role_label(role: &SyncRole) -> &'static str {
    match role {
        SyncRole::Leader => "leader",
        SyncRole::Follower => "follower",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MagGraphConfig, SyncRole};
    use crate::index::GraphIndex;
    use crate::node::{NewNode, NodeMetadata};
    use git2::Repository as GitRepo;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_config(dir: &Path, role: SyncRole, remote_url: &str) -> ResolvedConfig {
        let role_str = match role {
            SyncRole::Leader => "leader",
            SyncRole::Follower => "follower",
        };
        let contents = format!(
            r#"
[storage]
mode = "local"
root_path = "./graph"

[sync]
role = "{role_str}"
remote_url = "{remote_url}"
"#
        );
        let path = dir.join("maggraph.toml");
        std::fs::write(&path, contents).expect("write config");
        MagGraphConfig::load(&path).expect("load")
    }

    fn init_bare(path: &Path) -> GitRepo {
        let repo = GitRepo::init_bare(path).expect("bare");
        repo.reference_symbolic("HEAD", "refs/heads/main", true, "init default branch")
            .expect("set bare HEAD");
        repo
    }

    #[test]
    fn leader_push_follower_pull_readonly() {
        let temp = TempDir::new().expect("temp dir");
        let bare = temp.path().join("remote.git");
        init_bare(&bare);
        let remote_url = bare.to_string_lossy().to_string();

        let leader_dir = temp.path().join("leader_setup");
        std::fs::create_dir_all(&leader_dir).expect("mkdir");
        let leader_resolved = write_config(&leader_dir, SyncRole::Leader, &remote_url);
        std::fs::create_dir_all(&leader_resolved.root_path).expect("graph root");

        let mut leader_sync = SyncEngine::init(&leader_resolved).expect("init leader");

        leader_sync
            .with_leader_write("leader-test", |policy| {
                let mut index = GraphIndex::open(leader_sync.root_path()).expect("open index");
                index
                    .create_node_with_policy(
                        NewNode {
                            metadata: NodeMetadata {
                                id: "synced_node".into(),
                                node_type: "note".into(),
                                source: None,
                                links: vec![],
                                extra: BTreeMap::new(),
                            },
                            body: "# Synced\n".into(),
                            relative_path: PathBuf::from("synced_node.md"),
                        },
                        &policy,
                    )
                    .map(|_| ())
            })
            .expect("leader write");

        leader_sync
            .commit_and_push("Add synced node")
            .expect("push");

        let follower_dir = temp.path().join("follower_setup");
        std::fs::create_dir_all(&follower_dir).expect("mkdir");
        let follower_resolved = write_config(&follower_dir, SyncRole::Follower, &remote_url);

        let mut follower_sync =
            SyncEngine::clone_follower(&follower_resolved).expect("clone follower");
        follower_sync.pull().expect("pull");

        let follower_index = GraphIndex::open(follower_sync.root_path()).expect("follower index");
        assert!(follower_index.contains("synced_node"));

        let mut follower_index = follower_index;
        let err = follower_sync
            .with_leader_write("follower", |_| Ok(()))
            .expect_err("follower cannot use leader write");

        assert!(matches!(err, MagGraphError::ReadOnlyRole { .. }));

        let err = follower_index
            .create_node_with_policy(
                NewNode {
                    metadata: NodeMetadata {
                        id: "blocked".into(),
                        node_type: "note".into(),
                        source: None,
                        links: vec![],
                        extra: BTreeMap::new(),
                    },
                    body: "nope".into(),
                    relative_path: PathBuf::from("blocked.md"),
                },
                &follower_sync.write_policy(),
            )
            .expect_err("follower write blocked");

        assert!(matches!(err, MagGraphError::ReadOnlyRole { .. }));
    }

    #[test]
    fn follower_push_is_rejected() {
        let temp = TempDir::new().expect("temp dir");
        let bare = temp.path().join("remote.git");
        init_bare(&bare);
        let remote_url = bare.to_string_lossy().to_string();

        let leader_dir = temp.path().join("leader_setup");
        std::fs::create_dir_all(&leader_dir).expect("mkdir");
        let leader_resolved = write_config(&leader_dir, SyncRole::Leader, &remote_url);
        std::fs::create_dir_all(&leader_resolved.root_path).expect("graph root");
        let mut leader = SyncEngine::init(&leader_resolved).expect("init");
        leader.commit_and_push("seed").expect("seed push");

        let follower_dir = temp.path().join("follower_setup");
        std::fs::create_dir_all(&follower_dir).expect("mkdir");
        let follower_resolved = write_config(&follower_dir, SyncRole::Follower, &remote_url);
        let mut follower = SyncEngine::clone_follower(&follower_resolved).expect("clone");

        let err = follower.push().expect_err("follower push");
        assert!(matches!(err, MagGraphError::ReadOnlyRole { .. }));
    }
}
