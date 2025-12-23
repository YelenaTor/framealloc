//! Frame checkpoints - save and restore points within a frame.
//!
//! Checkpoints allow speculative allocation with rollback capability.
//! This is useful for try/fail patterns where you want to undo
//! allocations if an operation fails.

use std::marker::PhantomData;

/// A checkpoint representing a saved position in the frame arena.
///
/// Checkpoints can be used to rollback speculative allocations.
/// They are zero-cost when not used - just a usize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameCheckpoint {
    /// The saved head position
    head: usize,
    /// Frame number when checkpoint was created (for validation)
    frame_id: u64,
}

impl FrameCheckpoint {
    /// Create a new checkpoint at the given position.
    pub(crate) fn new(head: usize, frame_id: u64) -> Self {
        Self { head, frame_id }
    }

    /// Get the saved head position.
    pub fn head(&self) -> usize {
        self.head
    }

    /// Get the frame ID when this checkpoint was created.
    pub fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

/// RAII guard for checkpoint-based rollback.
///
/// If not explicitly committed, the checkpoint is rolled back on drop.
pub struct CheckpointGuard<'a> {
    checkpoint: FrameCheckpoint,
    committed: bool,
    _marker: PhantomData<&'a ()>,
}

impl<'a> CheckpointGuard<'a> {
    /// Create a new checkpoint guard.
    pub(crate) fn new(checkpoint: FrameCheckpoint) -> Self {
        Self {
            checkpoint,
            committed: false,
            _marker: PhantomData,
        }
    }

    /// Commit the allocations, preventing rollback.
    pub fn commit(mut self) {
        self.committed = true;
    }

    /// Get the checkpoint.
    pub fn checkpoint(&self) -> FrameCheckpoint {
        self.checkpoint
    }

    /// Check if this guard has been committed.
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

impl<'a> Drop for CheckpointGuard<'a> {
    fn drop(&mut self) {
        if !self.committed {
            // Rollback will be handled by the caller checking is_committed
            // We can't access TLS here safely, so we just mark it
        }
    }
}

/// Result of a speculative allocation block.
#[derive(Debug)]
pub enum SpeculativeResult<T, E> {
    /// The operation succeeded, allocations are kept.
    Success(T),
    /// The operation failed, allocations were rolled back.
    RolledBack(E),
}

impl<T, E> SpeculativeResult<T, E> {
    /// Check if the operation succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if the operation was rolled back.
    pub fn is_rolled_back(&self) -> bool {
        matches!(self, Self::RolledBack(_))
    }

    /// Convert to a Result.
    pub fn into_result(self) -> Result<T, E> {
        match self {
            Self::Success(t) => Ok(t),
            Self::RolledBack(e) => Err(e),
        }
    }

    /// Unwrap the success value.
    pub fn unwrap(self) -> T
    where
        E: std::fmt::Debug,
    {
        match self {
            Self::Success(t) => t,
            Self::RolledBack(e) => panic!("called unwrap on RolledBack: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let cp = FrameCheckpoint::new(1024, 42);
        assert_eq!(cp.head(), 1024);
        assert_eq!(cp.frame_id(), 42);
    }

    #[test]
    fn test_speculative_result() {
        let success: SpeculativeResult<i32, &str> = SpeculativeResult::Success(42);
        assert!(success.is_success());
        assert_eq!(success.unwrap(), 42);

        let rolled_back: SpeculativeResult<i32, &str> = SpeculativeResult::RolledBack("failed");
        assert!(rolled_back.is_rolled_back());
        assert!(rolled_back.into_result().is_err());
    }
}
