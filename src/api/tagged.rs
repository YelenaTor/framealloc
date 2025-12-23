//! First-class allocation tags - ergonomic tag-scoped allocations.
//!
//! Tags allow attributing allocations to subsystems for better
//! budgeting, profiling, and diagnostics.

use std::cell::RefCell;

/// A stack of active allocation tags for the current thread.
pub struct TagStack {
    stack: Vec<&'static str>,
}

impl TagStack {
    /// Create a new tag stack.
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(8),
        }
    }

    /// Push a tag onto the stack.
    pub fn push(&mut self, tag: &'static str) {
        self.stack.push(tag);
    }

    /// Pop the current tag.
    pub fn pop(&mut self) -> Option<&'static str> {
        self.stack.pop()
    }

    /// Get the current tag.
    pub fn current(&self) -> Option<&'static str> {
        self.stack.last().copied()
    }

    /// Check if a tag is active.
    pub fn is_active(&self, tag: &'static str) -> bool {
        self.stack.iter().any(|&t| t == tag)
    }

    /// Get the full tag path (for nested tags).
    pub fn path(&self) -> String {
        self.stack.join("::")
    }

    /// Clear all tags.
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Get the depth of the tag stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl Default for TagStack {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static TAG_STACK: RefCell<TagStack> = RefCell::new(TagStack::new());
}

/// Push a tag onto the current thread's tag stack.
pub fn push_tag(tag: &'static str) {
    TAG_STACK.with(|s| s.borrow_mut().push(tag));
}

/// Pop the current tag from the stack.
pub fn pop_tag() -> Option<&'static str> {
    TAG_STACK.with(|s| s.borrow_mut().pop())
}

/// Get the current tag.
pub fn current_tag() -> Option<&'static str> {
    TAG_STACK.with(|s| s.borrow().current())
}

/// Get the full tag path.
pub fn tag_path() -> String {
    TAG_STACK.with(|s| s.borrow().path())
}

/// Clear all tags.
pub fn clear_tags() {
    TAG_STACK.with(|s| s.borrow_mut().clear());
}

/// RAII guard for a tagged scope.
///
/// Automatically pops the tag when dropped.
pub struct TagGuard {
    tag: &'static str,
}

impl TagGuard {
    /// Create a new tag guard.
    pub fn new(tag: &'static str) -> Self {
        push_tag(tag);
        Self { tag }
    }

    /// Get the tag this guard is for.
    pub fn tag(&self) -> &'static str {
        self.tag
    }
}

impl Drop for TagGuard {
    fn drop(&mut self) {
        pop_tag();
    }
}

/// Execute a closure with a tag active.
///
/// The tag is automatically pushed before and popped after the closure.
pub fn with_tag<F, R>(tag: &'static str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = TagGuard::new(tag);
    f()
}

/// Allocation statistics by tag.
#[derive(Debug, Clone, Default)]
pub struct TaggedStats {
    /// Tag name
    pub tag: &'static str,
    /// Total bytes allocated under this tag
    pub bytes_allocated: usize,
    /// Total allocations under this tag
    pub allocation_count: usize,
    /// Current bytes (allocated - freed)
    pub current_bytes: usize,
}

impl TaggedStats {
    /// Create new stats for a tag.
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            bytes_allocated: 0,
            allocation_count: 0,
            current_bytes: 0,
        }
    }

    /// Record an allocation.
    pub fn record_alloc(&mut self, size: usize) {
        self.bytes_allocated += size;
        self.allocation_count += 1;
        self.current_bytes += size;
    }

    /// Record a deallocation.
    pub fn record_dealloc(&mut self, size: usize) {
        self.current_bytes = self.current_bytes.saturating_sub(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_stack() {
        clear_tags();

        push_tag("rendering");
        assert_eq!(current_tag(), Some("rendering"));

        push_tag("shadows");
        assert_eq!(current_tag(), Some("shadows"));
        assert_eq!(tag_path(), "rendering::shadows");

        pop_tag();
        assert_eq!(current_tag(), Some("rendering"));

        pop_tag();
        assert_eq!(current_tag(), None);
    }

    #[test]
    fn test_tag_guard() {
        clear_tags();

        {
            let _guard = TagGuard::new("physics");
            assert_eq!(current_tag(), Some("physics"));

            {
                let _inner = TagGuard::new("collision");
                assert_eq!(current_tag(), Some("collision"));
            }

            assert_eq!(current_tag(), Some("physics"));
        }

        assert_eq!(current_tag(), None);
    }

    #[test]
    fn test_with_tag() {
        clear_tags();

        let result = with_tag("ai", || {
            assert_eq!(current_tag(), Some("ai"));
            42
        });

        assert_eq!(result, 42);
        assert_eq!(current_tag(), None);
    }
}
