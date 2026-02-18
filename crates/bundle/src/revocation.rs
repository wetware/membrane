//! Revocation primitive for bundle grants.
//!
//! The searcher retains a [`RevocationHandle`] and can call [`revoke()`](RevocationHandle::revoke)
//! at any time. The [`RevocationGuard`] is shared with capability servers and checked on every
//! RPC call. Revocation is a one-way monotonic latch: once true, always true.

use capnp::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Guard that checks whether the bundle grant has been revoked.
/// Shared between the searcher's revocation handle and all
/// BundleAccess servers issued under this grant.
#[derive(Clone)]
pub struct RevocationGuard {
    revoked: Arc<AtomicBool>,
}

/// Handle retained by the searcher to revoke the grant.
/// Calling [`revoke()`](Self::revoke) is idempotent.
pub struct RevocationHandle {
    revoked: Arc<AtomicBool>,
}

impl RevocationGuard {
    /// Create a new revocation pair: handle (for the searcher) and guard (for capability servers).
    pub fn new() -> (RevocationHandle, Self) {
        let flag = Arc::new(AtomicBool::new(false));
        let handle = RevocationHandle {
            revoked: flag.clone(),
        };
        let guard = RevocationGuard { revoked: flag };
        (handle, guard)
    }

    /// Check whether the grant has been revoked.
    /// Returns `Ok(())` if still valid, `Err` if revoked.
    pub fn check(&self) -> Result<(), Error> {
        if self.revoked.load(Ordering::Acquire) {
            return Err(Error::failed(
                "revoked: bundle grant has been revoked".to_string(),
            ));
        }
        Ok(())
    }
}

impl RevocationHandle {
    /// Revoke the grant. Idempotent — calling multiple times is safe.
    pub fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
    }

    /// Check whether revocation has been triggered.
    pub fn is_revoked(&self) -> bool {
        self.revoked.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_passes_before_revocation() {
        let (_handle, guard) = RevocationGuard::new();
        assert!(guard.check().is_ok());
    }

    #[test]
    fn guard_fails_after_revocation() {
        let (handle, guard) = RevocationGuard::new();
        handle.revoke();
        let res = guard.check();
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("revoked"));
    }

    #[test]
    fn revoke_is_idempotent() {
        let (handle, guard) = RevocationGuard::new();
        handle.revoke();
        handle.revoke(); // no panic
        assert!(guard.check().is_err());
    }

    #[test]
    fn cloned_guard_sees_revocation() {
        let (handle, guard) = RevocationGuard::new();
        let guard2 = guard.clone();
        assert!(guard2.check().is_ok());
        handle.revoke();
        assert!(guard2.check().is_err());
    }
}
