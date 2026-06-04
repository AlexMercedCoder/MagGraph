use crate::config::SyncRole;
use crate::error::{MagGraphError, Result};
use crate::sync::lock::WriteLockGuard;

/// Write authorization policy derived from sync role and optional leader lock.
#[derive(Debug, Clone)]
pub struct WritePolicy<'a> {
    role: Option<SyncRole>,
    lock: Option<&'a WriteLockGuard>,
}

impl WritePolicy<'static> {
    /// No sync configured — all writes allowed.
    pub fn unrestricted() -> Self {
        Self {
            role: None,
            lock: None,
        }
    }

    /// Follower role — writes rejected.
    pub fn follower() -> Self {
        Self {
            role: Some(SyncRole::Follower),
            lock: None,
        }
    }
    /// Leader role without an acquired lock — writes rejected until lock is held.
    pub fn leader() -> Self {
        Self {
            role: Some(SyncRole::Leader),
            lock: None,
        }
    }

    pub fn from_role(role: SyncRole) -> Self {
        match role {
            SyncRole::Follower => Self::follower(),
            SyncRole::Leader => Self::leader(),
        }
    }
}

impl<'a> WritePolicy<'a> {
    pub fn leader_with_lock(lock: &'a WriteLockGuard) -> Self {
        Self {
            role: Some(SyncRole::Leader),
            lock: Some(lock),
        }
    }

    pub fn assert_can_write(&self) -> Result<()> {
        if self.role == Some(SyncRole::Follower) {
            return Err(MagGraphError::ReadOnlyRole {
                role: "follower".into(),
            });
        }

        if self.role == Some(SyncRole::Leader) && self.lock.is_none() {
            return Err(MagGraphError::WriteLockRequired);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::lock::WriteLockGuard;
    use tempfile::TempDir;

    #[test]
    fn follower_rejects_writes() {
        let err = WritePolicy::follower()
            .assert_can_write()
            .expect_err("follower write");
        assert!(matches!(err, MagGraphError::ReadOnlyRole { .. }));
    }

    #[test]
    fn leader_without_lock_rejects_writes() {
        let err = WritePolicy::from_role(SyncRole::Leader)
            .assert_can_write()
            .expect_err("leader without lock");
        assert!(matches!(err, MagGraphError::WriteLockRequired));
    }

    #[test]
    fn leader_with_lock_allows_writes() {
        let temp = TempDir::new().expect("temp dir");
        let lock = WriteLockGuard::acquire(temp.path(), "leader").expect("lock");
        WritePolicy::leader_with_lock(&lock)
            .assert_can_write()
            .expect("allowed");
    }
}
