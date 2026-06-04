use std::path::{Path, PathBuf};

use git2::{
    build::RepoBuilder, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository,
    Signature, Status, StatusOptions,
};

use crate::config::METADATA_DIR_NAME;
use crate::error::{MagGraphError, Result};

pub const DEFAULT_REMOTE: &str = "origin";
pub const DEFAULT_BRANCH: &str = "main";

/// Summary of the working tree and upstream tracking state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatus {
    pub branch: String,
    pub uncommitted: usize,
    pub ahead: usize,
    pub behind: usize,
    pub clean: bool,
}

/// Result of a pull (fetch + merge) operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullResult {
    pub merged: bool,
    pub fast_forward: bool,
    pub conflicts: Vec<PathBuf>,
}

/// Result of a push operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushResult {
    pub pushed: bool,
    pub commit: Option<String>,
}

pub struct GitRepository {
    repo: Repository,
    root_path: PathBuf,
}

impl GitRepository {
    pub fn open(root_path: impl AsRef<Path>) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        let repo = Repository::open(&root_path).map_err(|source| MagGraphError::Git {
            message: format!(
                "failed to open repository at {}: {source}",
                root_path.display()
            ),
        })?;

        Ok(Self { repo, root_path })
    }

    pub fn init(root_path: impl AsRef<Path>) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root_path).map_err(|source| MagGraphError::Git {
            message: format!(
                "failed to create graph root {}: {source}",
                root_path.display()
            ),
        })?;

        let repo = if root_path.join(".git").exists() {
            Repository::open(&root_path).map_err(|source| MagGraphError::Git {
                message: format!("failed to open existing repository: {source}"),
            })?
        } else {
            Repository::init(&root_path).map_err(|source| MagGraphError::Git {
                message: format!(
                    "failed to init repository at {}: {source}",
                    root_path.display()
                ),
            })?
        };

        Ok(Self { repo, root_path })
    }

    pub fn clone_from(url: &str, root_path: impl AsRef<Path>) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        let repo = RepoBuilder::new()
            .branch(DEFAULT_BRANCH)
            .clone(url, &root_path)
            .map_err(|source| MagGraphError::Git {
                message: format!(
                    "failed to clone {url} into {}: {source}",
                    root_path.display()
                ),
            })?;

        let git = Self { repo, root_path };
        git.ensure_head()?;
        Ok(git)
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn inner(&self) -> &Repository {
        &self.repo
    }

    pub fn ensure_remote(&mut self, name: &str, url: &str) -> Result<()> {
        if self.repo.find_remote(name).is_ok() {
            self.repo
                .remote_set_url(name, url)
                .map_err(|source| MagGraphError::Git {
                    message: format!("failed to update remote `{name}` url: {source}"),
                })?;
            return Ok(());
        }

        self.repo
            .remote(name, url)
            .map_err(|source| MagGraphError::Git {
                message: format!("failed to add remote `{name}` -> {url}: {source}"),
            })?;
        Ok(())
    }

    pub fn ensure_initial_commit(&mut self) -> Result<()> {
        if self.repo.head().is_ok() {
            return Ok(());
        }

        self.write_default_gitignore()?;

        let mut index = self.repo.index().map_err(git_error)?;
        if index.is_empty() {
            index
                .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
                .map_err(git_error)?;
            index.write().map_err(git_error)?;
        }

        let tree_id = index.write_tree().map_err(git_error)?;
        let tree = self.repo.find_tree(tree_id).map_err(git_error)?;
        let sig = default_signature(&self.repo)?;

        let branch_ref = branch_ref_name();

        self.repo
            .commit(
                Some(&branch_ref),
                &sig,
                &sig,
                "Initial MagGraph commit",
                &tree,
                &[],
            )
            .map_err(git_error)?;

        self.repo.set_head(&branch_ref).map_err(git_error)?;
        self.repo.checkout_head(None).map_err(git_error)?;

        Ok(())
    }

    pub fn status(&self) -> Result<SyncStatus> {
        let branch = current_branch(&self.repo).unwrap_or_else(|| DEFAULT_BRANCH.to_string());

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .exclude_submodules(true);

        let statuses = self.repo.statuses(Some(&mut opts)).map_err(git_error)?;
        let uncommitted = statuses
            .iter()
            .filter(|entry| {
                let s = entry.status();
                !s.is_ignored()
                    && (s.contains(Status::INDEX_NEW)
                        | s.contains(Status::INDEX_MODIFIED)
                        | s.contains(Status::WT_NEW)
                        | s.contains(Status::WT_MODIFIED)
                        | s.contains(Status::WT_DELETED)
                        | s.contains(Status::INDEX_DELETED))
            })
            .count();

        let (ahead, behind) = upstream_ahead_behind(&self.repo).unwrap_or((0, 0));

        Ok(SyncStatus {
            branch,
            uncommitted,
            ahead,
            behind,
            clean: uncommitted == 0 && ahead == 0 && behind == 0,
        })
    }

    pub fn commit_all(&mut self, message: &str) -> Result<Option<String>> {
        let mut index = self.repo.index().map_err(git_error)?;
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(git_error)?;
        index.write().map_err(git_error)?;

        if index.is_empty() {
            return Ok(None);
        }

        let tree_id = index.write_tree().map_err(git_error)?;
        let tree = self.repo.find_tree(tree_id).map_err(git_error)?;
        let sig = default_signature(&self.repo)?;

        let parent = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok());

        let oid = if let Some(parent) = parent {
            self.repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .map_err(git_error)?
        } else {
            self.repo
                .commit(Some(&branch_ref_name()), &sig, &sig, message, &tree, &[])
                .map_err(git_error)?
        };

        self.ensure_head()?;

        Ok(Some(oid.to_string()))
    }

    pub fn pull(&mut self, remote_name: &str) -> Result<PullResult> {
        self.ensure_head()?;
        self.fetch(remote_name)?;

        let head = self
            .repo
            .head()
            .map_err(|_| MagGraphError::Git {
                message: "repository has no HEAD; create an initial commit before pulling".into(),
            })?
            .peel_to_commit()
            .map_err(git_error)?;

        let remote_branch = format!("refs/remotes/{remote_name}/{DEFAULT_BRANCH}");
        let remote_commit = self
            .repo
            .find_reference(&remote_branch)
            .map_err(|_| MagGraphError::Git {
                message: format!(
                    "remote branch `{remote_name}/{DEFAULT_BRANCH}` not found after fetch"
                ),
            })?
            .peel_to_commit()
            .map_err(git_error)?;

        if head.id() == remote_commit.id() {
            return Ok(PullResult {
                merged: false,
                fast_forward: false,
                conflicts: vec![],
            });
        }

        let annotated = self
            .repo
            .find_annotated_commit(remote_commit.id())
            .map_err(git_error)?;

        let (merge_result, _) = self.repo.merge_analysis(&[&annotated]).map_err(git_error)?;

        if merge_result.is_up_to_date() {
            return Ok(PullResult {
                merged: false,
                fast_forward: false,
                conflicts: vec![],
            });
        }

        if merge_result.is_fast_forward() {
            let refname = branch_ref_name();
            self.repo
                .find_reference(&refname)
                .map_err(git_error)?
                .set_target(remote_commit.id(), "Fast-forward merge")
                .map_err(git_error)?;
            self.repo.checkout_head(None).map_err(git_error)?;
            self.repo.set_head(&refname).map_err(git_error)?;

            return Ok(PullResult {
                merged: true,
                fast_forward: true,
                conflicts: vec![],
            });
        }

        if merge_result.is_normal() {
            self.repo
                .merge(&[&annotated], None, None)
                .map_err(git_error)?;

            let mut index = self.repo.index().map_err(git_error)?;
            if index.has_conflicts() {
                let conflicts = collect_conflicts(&index);
                return Ok(PullResult {
                    merged: false,
                    fast_forward: false,
                    conflicts,
                });
            }

            let tree_id = index.write_tree_to(&self.repo).map_err(git_error)?;
            let tree = self.repo.find_tree(tree_id).map_err(git_error)?;
            let sig = default_signature(&self.repo)?;

            self.repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("Merge {remote_name}/{DEFAULT_BRANCH}"),
                    &tree,
                    &[&head, &remote_commit],
                )
                .map_err(git_error)?;

            self.repo.checkout_head(None).map_err(git_error)?;

            return Ok(PullResult {
                merged: true,
                fast_forward: false,
                conflicts: vec![],
            });
        }

        Err(MagGraphError::Git {
            message: "unsupported merge analysis result".into(),
        })
    }

    pub fn push(&mut self, remote_name: &str) -> Result<PushResult> {
        let head = self.repo.head().map_err(|_| MagGraphError::Git {
            message: "repository has no HEAD; nothing to push".into(),
        })?;
        let local_ref = head.name().ok_or_else(|| MagGraphError::Git {
            message: "HEAD is detached or unborn".into(),
        })?;

        let remote_ref = branch_ref_name();
        let refspec = format!("{local_ref}:{remote_ref}");

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed| {
            git2::Cred::username(username_from_url.unwrap_or("git"))
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        self.repo
            .find_remote(remote_name)
            .map_err(|source| MagGraphError::Git {
                message: format!("remote `{remote_name}` not found: {source}"),
            })?
            .push(&[&refspec], Some(&mut push_options))
            .map_err(git_error)?;

        let commit = head.peel_to_commit().ok().map(|c| c.id().to_string());

        Ok(PushResult {
            pushed: true,
            commit,
        })
    }

    fn fetch(&mut self, remote_name: &str) -> Result<()> {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed| {
            git2::Cred::username(username_from_url.unwrap_or("git"))
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        self.repo
            .find_remote(remote_name)
            .map_err(|source| MagGraphError::Git {
                message: format!("remote `{remote_name}` not found: {source}"),
            })?
            .fetch(
                &[DEFAULT_BRANCH],
                Some(&mut fetch_options),
                Some("Fetching origin"),
            )
            .map_err(git_error)?;

        Ok(())
    }

    fn write_default_gitignore(&self) -> Result<()> {
        let gitignore = self.root_path.join(".gitignore");
        if gitignore.exists() {
            return Ok(());
        }

        let contents = format!("# MagGraph local metadata (not synced)\n{METADATA_DIR_NAME}/\n");
        std::fs::write(&gitignore, contents).map_err(|source| MagGraphError::Git {
            message: format!("failed to write .gitignore: {source}"),
        })?;
        Ok(())
    }

    fn ensure_head(&self) -> Result<()> {
        if self.repo.head().is_ok() {
            return Ok(());
        }

        let branch = branch_ref_name();
        if self.repo.find_reference(&branch).is_ok() {
            self.repo.set_head(&branch).map_err(git_error)?;
            self.repo.checkout_head(None).map_err(git_error)?;
        }

        Ok(())
    }
}

fn branch_ref_name() -> String {
    format!("refs/heads/{DEFAULT_BRANCH}")
}

fn git_error(source: git2::Error) -> MagGraphError {
    MagGraphError::Git {
        message: source.to_string(),
    }
}

fn default_signature(repo: &Repository) -> Result<Signature<'static>> {
    repo.signature()
        .or_else(|_| Signature::now("maggraph", "maggraph@localhost"))
        .map_err(git_error)
}

fn current_branch(repo: &Repository) -> Option<String> {
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(str::to_string))
}

fn upstream_ahead_behind(repo: &Repository) -> Result<(usize, usize)> {
    let head = repo.head().map_err(git_error)?;
    let local_oid = head.target().ok_or_else(|| MagGraphError::Git {
        message: "HEAD has no target".into(),
    })?;

    let branch_name = head.shorthand().unwrap_or(DEFAULT_BRANCH);
    let upstream_name = format!("refs/remotes/{DEFAULT_REMOTE}/{branch_name}");

    let upstream_oid = match repo.find_reference(&upstream_name) {
        Ok(reference) => reference.target().ok_or_else(|| MagGraphError::Git {
            message: format!("upstream `{upstream_name}` has no target"),
        })?,
        Err(_) => return Ok((0, 0)),
    };

    let (ahead, behind) = repo
        .graph_ahead_behind(local_oid, upstream_oid)
        .map_err(git_error)?;

    Ok((ahead, behind))
}

fn collect_conflicts(index: &git2::Index) -> Vec<PathBuf> {
    index
        .conflicts()
        .map(|conflicts| {
            conflicts
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    entry
                        .ancestor
                        .or(entry.our)
                        .or(entry.their)
                        .map(|e| PathBuf::from(String::from_utf8_lossy(&e.path).into_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository as GitRepo;
    use tempfile::TempDir;

    fn init_bare_remote(path: &Path) -> GitRepo {
        let repo = GitRepo::init_bare(path).expect("init bare");
        repo.reference_symbolic("HEAD", &branch_ref_name(), true, "init default branch")
            .expect("set bare HEAD");
        repo
    }

    #[test]
    fn init_commit_and_status() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        std::fs::create_dir_all(&root).expect("mkdir");

        std::fs::write(root.join("welcome.md"), "# Welcome\n").expect("write");

        let mut repo = GitRepository::init(&root).expect("init");
        repo.ensure_initial_commit().expect("initial commit");

        let status = repo.status().expect("status");
        assert_eq!(status.uncommitted, 0);
        assert!(root.join(".gitignore").is_file());
    }

    #[test]
    fn push_and_pull_between_clones() {
        let temp = TempDir::new().expect("temp dir");
        let bare_path = temp.path().join("remote.git");
        init_bare_remote(&bare_path);
        let remote_url = bare_path.to_string_lossy().to_string();

        let leader_root = temp.path().join("leader");
        std::fs::create_dir_all(&leader_root).expect("mkdir leader");
        std::fs::write(
            leader_root.join("node.md"),
            "---\nid: \"n1\"\ntype: \"note\"\n---\n# Node\n",
        )
        .expect("write node");

        let mut leader = GitRepository::init(&leader_root).expect("init leader");
        leader
            .ensure_remote(DEFAULT_REMOTE, &remote_url)
            .expect("remote");
        leader.ensure_initial_commit().expect("commit");
        leader.push(DEFAULT_REMOTE).expect("push");

        let follower_root = temp.path().join("follower");
        let mut follower = GitRepository::clone_from(&remote_url, &follower_root).expect("clone");
        follower
            .ensure_remote(DEFAULT_REMOTE, &remote_url)
            .expect("remote");

        let pull = follower.pull(DEFAULT_REMOTE).expect("pull");
        assert!(pull.merged || pull.fast_forward || pull.conflicts.is_empty());
        assert!(follower_root.join("node.md").is_file());
    }

    #[test]
    fn merge_conflict_surfaces_paths() {
        let temp = TempDir::new().expect("temp dir");
        let bare_path = temp.path().join("remote.git");
        init_bare_remote(&bare_path);
        let remote_url = bare_path.to_string_lossy().to_string();

        let base_root = temp.path().join("base");
        std::fs::create_dir_all(&base_root).expect("mkdir");
        std::fs::write(base_root.join("shared.md"), "base\n").expect("write");

        let mut base = GitRepository::init(&base_root).expect("init");
        base.ensure_remote(DEFAULT_REMOTE, &remote_url)
            .expect("remote");
        base.ensure_initial_commit().expect("commit");
        base.push(DEFAULT_REMOTE).expect("push");

        let left_root = temp.path().join("left");
        let mut left = GitRepository::clone_from(&remote_url, &left_root).expect("clone left");
        left.ensure_remote(DEFAULT_REMOTE, &remote_url)
            .expect("remote");

        let right_root = temp.path().join("right");
        let mut right = GitRepository::clone_from(&remote_url, &right_root).expect("clone right");
        right
            .ensure_remote(DEFAULT_REMOTE, &remote_url)
            .expect("remote");

        std::fs::write(left_root.join("shared.md"), "left edit\n").expect("write left");
        left.commit_all("left change").expect("commit left");
        left.push(DEFAULT_REMOTE).expect("push left");

        std::fs::write(right_root.join("shared.md"), "right edit\n").expect("write right");
        right.commit_all("right change").expect("commit right");

        let pull = right.pull(DEFAULT_REMOTE).expect("pull");
        assert!(!pull.conflicts.is_empty());
        assert!(pull.conflicts.iter().any(|p| p.ends_with("shared.md")));
    }
}
