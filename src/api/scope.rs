//! Frame scope guards for RAII-style frame management.

use crate::api::alloc::SmartAlloc;
use crate::core::tls;

/// A guard that represents a frame scope.
///
/// When dropped, the frame arena is reset to its state when the guard was created.
/// This is useful for sub-frame temporary allocations.
///
/// # Example
///
/// ```rust,no_run
/// use framealloc::SmartAlloc;
///
/// let alloc = SmartAlloc::with_defaults();
///
/// {
///     let _scope = alloc.frame_scope();
///     let temp = alloc.frame_alloc::<[u8; 1024]>();
///     // temp is valid here
/// } // temp is invalidated here
/// ```
pub struct FrameGuard<'a> {
    alloc: &'a SmartAlloc,
    saved_head: usize,
}

impl<'a> FrameGuard<'a> {
    /// Create a new frame guard.
    pub(crate) fn new(alloc: &'a SmartAlloc) -> Self {
        let saved_head = tls::with_tls(|tls| tls.frame_head());
        Self { alloc, saved_head }
    }

    /// Allocate from this scope's frame arena.
    pub fn alloc<T>(&self) -> *mut T {
        self.alloc.frame_alloc::<T>()
    }
}

impl<'a> Drop for FrameGuard<'a> {
    fn drop(&mut self) {
        tls::with_tls(|tls| {
            tls.reset_frame_to(self.saved_head);
        });
    }
}

/// A trait for types that can provide a frame scope.
pub trait FrameScope {
    /// Create a new frame scope.
    fn frame_scope(&self) -> FrameGuard<'_>;
}

impl FrameScope for SmartAlloc {
    fn frame_scope(&self) -> FrameGuard<'_> {
        FrameGuard::new(self)
    }
}
