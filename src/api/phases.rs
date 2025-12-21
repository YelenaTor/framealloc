//! Frame phases - named scopes within a frame for better diagnostics and profiling.
//!
//! Phases divide a frame into logical sections (input, physics, render, etc.)
//! without changing allocation semantics. They integrate with diagnostics
//! and profiling for better visibility.

use std::cell::RefCell;

/// A named phase within a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    /// Name of the phase
    pub name: &'static str,
    /// Bytes allocated during this phase
    pub bytes_allocated: usize,
    /// Number of allocations in this phase
    pub allocation_count: usize,
    /// Start time (if timing enabled)
    #[cfg(feature = "diagnostics")]
    pub start_time: Option<std::time::Instant>,
}

impl Phase {
    /// Create a new phase.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            bytes_allocated: 0,
            allocation_count: 0,
            #[cfg(feature = "diagnostics")]
            start_time: Some(std::time::Instant::now()),
        }
    }

    /// Record an allocation in this phase.
    pub fn record_alloc(&mut self, size: usize) {
        self.bytes_allocated += size;
        self.allocation_count += 1;
    }

    /// Get phase duration (if timing enabled).
    #[cfg(feature = "diagnostics")]
    pub fn duration(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }
}

/// Tracks the current phase stack for a thread.
pub struct PhaseTracker {
    /// Stack of active phases (supports nesting)
    stack: Vec<Phase>,
    /// Completed phases this frame
    completed: Vec<Phase>,
}

impl PhaseTracker {
    /// Create a new phase tracker.
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(8),
            completed: Vec::with_capacity(16),
        }
    }

    /// Begin a new phase.
    pub fn begin_phase(&mut self, name: &'static str) {
        self.stack.push(Phase::new(name));
    }

    /// End the current phase.
    pub fn end_phase(&mut self) -> Option<Phase> {
        if let Some(phase) = self.stack.pop() {
            self.completed.push(phase.clone());
            Some(phase)
        } else {
            None
        }
    }

    /// Get the current phase name.
    pub fn current_phase(&self) -> Option<&'static str> {
        self.stack.last().map(|p| p.name)
    }

    /// Record an allocation in the current phase.
    pub fn record_alloc(&mut self, size: usize) {
        if let Some(phase) = self.stack.last_mut() {
            phase.record_alloc(size);
        }
    }

    /// Get completed phases this frame.
    pub fn completed_phases(&self) -> &[Phase] {
        &self.completed
    }

    /// Reset for a new frame.
    pub fn reset(&mut self) {
        self.stack.clear();
        self.completed.clear();
    }

    /// Check if any phase is active.
    pub fn is_in_phase(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Get the phase stack depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl Default for PhaseTracker {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static PHASE_TRACKER: RefCell<PhaseTracker> = RefCell::new(PhaseTracker::new());
}

/// Begin a named phase within the current frame.
pub fn begin_phase(name: &'static str) {
    PHASE_TRACKER.with(|t| t.borrow_mut().begin_phase(name));
}

/// End the current phase.
pub fn end_phase() -> Option<Phase> {
    PHASE_TRACKER.with(|t| t.borrow_mut().end_phase())
}

/// Get the current phase name.
pub fn current_phase() -> Option<&'static str> {
    PHASE_TRACKER.with(|t| t.borrow().current_phase())
}

/// Record an allocation in the current phase.
pub fn record_phase_alloc(size: usize) {
    PHASE_TRACKER.with(|t| t.borrow_mut().record_alloc(size));
}

/// Reset phases for a new frame.
pub fn reset_phases() {
    PHASE_TRACKER.with(|t| t.borrow_mut().reset());
}

/// Check if currently in a phase.
pub fn is_in_phase() -> bool {
    PHASE_TRACKER.with(|t| t.borrow().is_in_phase())
}

/// RAII guard for a phase scope.
pub struct PhaseGuard {
    _private: (),
}

impl PhaseGuard {
    /// Create a new phase guard.
    pub fn new(name: &'static str) -> Self {
        begin_phase(name);
        Self { _private: () }
    }
}

impl Drop for PhaseGuard {
    fn drop(&mut self) {
        end_phase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_tracking() {
        reset_phases();

        begin_phase("physics");
        assert_eq!(current_phase(), Some("physics"));

        record_phase_alloc(1024);
        record_phase_alloc(512);

        let phase = end_phase().unwrap();
        assert_eq!(phase.name, "physics");
        assert_eq!(phase.bytes_allocated, 1536);
        assert_eq!(phase.allocation_count, 2);
    }

    #[test]
    fn test_nested_phases() {
        reset_phases();

        begin_phase("update");
        begin_phase("physics");
        assert_eq!(current_phase(), Some("physics"));

        end_phase();
        assert_eq!(current_phase(), Some("update"));

        end_phase();
        assert_eq!(current_phase(), None);
    }

    #[test]
    fn test_phase_guard() {
        reset_phases();

        {
            let _guard = PhaseGuard::new("render");
            assert_eq!(current_phase(), Some("render"));
        }

        assert_eq!(current_phase(), None);
    }
}
