use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::METADATA_DIR_NAME;
use crate::error::{MagGraphError, Result};

/// Filename for the leader write lock under `.maggraph/`.
pub const LOCK_FILE_NAME: &str = "lock.toml";

/// On-disk lock record serialized as TOML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockRecord {
    pub holder: String,
    pub acquired_at: DateTime<Utc>,
    pub token: String,
}

/// RAII guard for the leader write lock.
#[derive(Debug)]
pub struct WriteLockGuard {
    lock_path: PathBuf,
    token: String,
    released: bool,
}

impl WriteLockGuard {
    /// Acquire the write lock for `holder`, creating `.maggraph/` if needed.
    pub fn acquire(root_path: &Path, holder: &str) -> Result<Self> {
        let metadata_dir = root_path.join(METADATA_DIR_NAME);
        fs::create_dir_all(&metadata_dir).map_err(|source| MagGraphError::WriteLock {
            path: metadata_dir.clone(),
            message: format!("failed to create metadata directory: {source}"),
        })?;

        let lock_path = metadata_dir.join(LOCK_FILE_NAME);

        if lock_path.exists() {
            let existing = read_lock(&lock_path)?;
            return Err(MagGraphError::WriteLockHeld {
                holder: existing.holder,
                acquired_at: existing.acquired_at.to_rfc3339(),
            });
        }

        let token = uuid::Uuid::new_v4().to_string();
        let record = LockRecord {
            holder: holder.to_string(),
            acquired_at: Utc::now(),
            token: token.clone(),
        };

        write_lock(&lock_path, &record)?;

        Ok(Self {
            lock_path,
            token,
            released: false,
        })
    }

    /// Release the lock if still held by this guard.
    pub fn release(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }

        if self.lock_path.exists() {
            let existing = read_lock(&self.lock_path)?;
            if existing.token != self.token {
                return Err(MagGraphError::WriteLock {
                    path: self.lock_path.clone(),
                    message: "lock token mismatch; another holder may have replaced the lock"
                        .into(),
                });
            }
            fs::remove_file(&self.lock_path).map_err(|source| MagGraphError::WriteLock {
                path: self.lock_path.clone(),
                message: format!("failed to remove lock file: {source}"),
            })?;
        }

        self.released = true;
        Ok(())
    }

    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for WriteLockGuard {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

fn read_lock(path: &Path) -> Result<LockRecord> {
    let raw = fs::read_to_string(path).map_err(|source| MagGraphError::WriteLock {
        path: path.to_path_buf(),
        message: format!("failed to read lock file: {source}"),
    })?;

    toml::from_str(&raw).map_err(|source| MagGraphError::WriteLock {
        path: path.to_path_buf(),
        message: format!("failed to parse lock file: {source}"),
    })
}

fn write_lock(path: &Path, record: &LockRecord) -> Result<()> {
    let raw = toml::to_string_pretty(record).map_err(|source| MagGraphError::WriteLock {
        path: path.to_path_buf(),
        message: format!("failed to serialize lock file: {source}"),
    })?;

    fs::write(path, raw).map_err(|source| MagGraphError::WriteLock {
        path: path.to_path_buf(),
        message: format!("failed to write lock file: {source}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_and_release_lock() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();

        let mut guard = WriteLockGuard::acquire(root, "leader-1").expect("acquire");
        assert!(guard.lock_path().is_file());

        guard.release().expect("release");
        assert!(!guard.lock_path().exists());
    }

    #[test]
    fn second_acquire_fails_while_held() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();

        let _guard = WriteLockGuard::acquire(root, "leader-1").expect("acquire");
        let err = WriteLockGuard::acquire(root, "leader-2").expect_err("held");
        assert!(matches!(err, MagGraphError::WriteLockHeld { .. }));
    }

    #[test]
    fn lock_released_on_drop() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let lock_path = root.join(METADATA_DIR_NAME).join(LOCK_FILE_NAME);

        {
            let _guard = WriteLockGuard::acquire(root, "leader-1").expect("acquire");
            assert!(lock_path.is_file());
        }

        assert!(!lock_path.exists());
    }
}
