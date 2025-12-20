//! Tracy profiler integration for memory visualization.
//!
//! When the `tracy` feature is enabled, this module provides hooks
//! for visualizing memory allocations in Tracy.

/// Tracy zone for memory allocations.
#[cfg(feature = "tracy")]
pub use tracy_client;

/// Trait for Tracy integration.
pub trait TracyIntegration {
    /// Mark a frame boundary for Tracy.
    fn tracy_frame_mark(&self);
    
    /// Create a memory allocation zone.
    fn tracy_alloc(&self, ptr: *const u8, size: usize, name: &str);
    
    /// Mark a memory free.
    fn tracy_free(&self, ptr: *const u8);
}

/// Memory event for external profilers.
#[derive(Debug, Clone)]
pub enum MemoryEvent {
    /// Memory was allocated
    Alloc {
        ptr: usize,
        size: usize,
        tag: Option<&'static str>,
    },
    /// Memory was freed
    Free {
        ptr: usize,
    },
    /// Frame boundary
    FrameMark {
        frame_number: u64,
    },
    /// Memory zone begin
    ZoneBegin {
        name: &'static str,
    },
    /// Memory zone end
    ZoneEnd,
}

/// Callback type for external profiler integration.
pub type ProfilerCallback = Box<dyn Fn(MemoryEvent) + Send + Sync>;

/// Profiler hooks for external tools.
pub struct ProfilerHooks {
    callback: Option<ProfilerCallback>,
    enabled: bool,
}

impl ProfilerHooks {
    /// Create new profiler hooks.
    pub fn new() -> Self {
        Self {
            callback: None,
            enabled: false,
        }
    }

    /// Set the profiler callback.
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(MemoryEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(callback));
        self.enabled = true;
    }

    /// Enable or disable profiling.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if profiling is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.callback.is_some()
    }

    /// Emit an allocation event.
    pub fn emit_alloc(&self, ptr: *const u8, size: usize, tag: Option<&'static str>) {
        if let Some(ref callback) = self.callback {
            if self.enabled {
                callback(MemoryEvent::Alloc {
                    ptr: ptr as usize,
                    size,
                    tag,
                });
            }
        }
    }

    /// Emit a free event.
    pub fn emit_free(&self, ptr: *const u8) {
        if let Some(ref callback) = self.callback {
            if self.enabled {
                callback(MemoryEvent::Free {
                    ptr: ptr as usize,
                });
            }
        }
    }

    /// Emit a frame mark.
    pub fn emit_frame_mark(&self, frame_number: u64) {
        if let Some(ref callback) = self.callback {
            if self.enabled {
                callback(MemoryEvent::FrameMark { frame_number });
            }
        }
    }

    /// Emit a zone begin.
    pub fn emit_zone_begin(&self, name: &'static str) {
        if let Some(ref callback) = self.callback {
            if self.enabled {
                callback(MemoryEvent::ZoneBegin { name });
            }
        }
    }

    /// Emit a zone end.
    pub fn emit_zone_end(&self) {
        if let Some(ref callback) = self.callback {
            if self.enabled {
                callback(MemoryEvent::ZoneEnd);
            }
        }
    }
}

impl Default for ProfilerHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for profiler zones.
pub struct ProfilerZone<'a> {
    hooks: &'a ProfilerHooks,
}

impl<'a> ProfilerZone<'a> {
    /// Create a new profiler zone.
    pub fn new(hooks: &'a ProfilerHooks, name: &'static str) -> Self {
        hooks.emit_zone_begin(name);
        Self { hooks }
    }
}

impl<'a> Drop for ProfilerZone<'a> {
    fn drop(&mut self) {
        self.hooks.emit_zone_end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_profiler_hooks() {
        let mut hooks = ProfilerHooks::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        hooks.set_callback(move |_event| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        hooks.emit_alloc(std::ptr::null(), 100, None);
        hooks.emit_free(std::ptr::null());
        hooks.emit_frame_mark(1);

        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_disabled_hooks() {
        let mut hooks = ProfilerHooks::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        hooks.set_callback(move |_event| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        hooks.set_enabled(false);
        hooks.emit_alloc(std::ptr::null(), 100, None);

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
