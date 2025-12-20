//! Diagnostic context - Bevy, thread, and frame state awareness.
//!
//! Provides context for more intelligent diagnostic messages.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::ThreadId;

/// Global frame counter for context.
static FRAME_NUMBER: AtomicU64 = AtomicU64::new(0);

/// Whether we're in a Bevy context.
static IS_BEVY_CONTEXT: AtomicBool = AtomicBool::new(false);

/// Diagnostic context containing runtime state.
#[derive(Debug, Clone)]
pub struct DiagContext {
    /// Whether Bevy integration is active.
    pub is_bevy: bool,
    /// Whether a frame is currently active.
    pub frame_active: bool,
    /// Current frame number (if known).
    pub frame_number: u64,
    /// Current thread ID.
    pub thread_id: ThreadId,
    /// Thread name (if available).
    pub thread_name: Option<String>,
    /// Whether this is the main thread.
    pub is_main_thread: bool,
}

impl DiagContext {
    /// Capture the current context.
    pub fn capture() -> Self {
        let thread = std::thread::current();
        let thread_name = thread.name().map(String::from);
        
        Self {
            is_bevy: IS_BEVY_CONTEXT.load(Ordering::Relaxed),
            frame_active: crate::core::tls::with_tls(|tls| tls.is_frame_active()),
            frame_number: FRAME_NUMBER.load(Ordering::Relaxed),
            thread_id: thread.id(),
            thread_name,
            is_main_thread: is_main_thread(),
        }
    }

    /// Create a minimal context (for when full capture isn't needed).
    pub fn minimal() -> Self {
        Self {
            is_bevy: IS_BEVY_CONTEXT.load(Ordering::Relaxed),
            frame_active: false,
            frame_number: FRAME_NUMBER.load(Ordering::Relaxed),
            thread_id: std::thread::current().id(),
            thread_name: None,
            is_main_thread: false,
        }
    }

    /// Format context for diagnostic output.
    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        if self.is_bevy {
            parts.push("bevy=true".to_string());
        }

        parts.push(format!("frame={}", self.frame_number));
        
        if self.frame_active {
            parts.push("frame_active=true".to_string());
        }

        if let Some(ref name) = self.thread_name {
            parts.push(format!("thread=\"{}\"", name));
        } else {
            parts.push(format!("thread={:?}", self.thread_id));
        }

        if self.is_main_thread {
            parts.push("main_thread=true".to_string());
        }

        parts.join(", ")
    }
}

impl std::fmt::Display for DiagContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

// =============================================================================
// Context management
// =============================================================================

/// Mark that Bevy integration is active.
pub fn set_bevy_context(active: bool) {
    IS_BEVY_CONTEXT.store(active, Ordering::Relaxed);
}

/// Check if Bevy context is active.
pub fn is_bevy_context() -> bool {
    IS_BEVY_CONTEXT.load(Ordering::Relaxed)
}

/// Increment the frame counter.
pub fn increment_frame() {
    FRAME_NUMBER.fetch_add(1, Ordering::Relaxed);
}

/// Get the current frame number.
pub fn frame_number() -> u64 {
    FRAME_NUMBER.load(Ordering::Relaxed)
}

/// Reset frame counter (for testing).
pub fn reset_frame_counter() {
    FRAME_NUMBER.store(0, Ordering::Relaxed);
}

// =============================================================================
// Thread detection
// =============================================================================

/// Cached main thread ID.
static MAIN_THREAD_ID: std::sync::OnceLock<ThreadId> = std::sync::OnceLock::new();

/// Initialize the main thread ID (call from main).
pub fn init_main_thread() {
    let _ = MAIN_THREAD_ID.set(std::thread::current().id());
}

/// Check if current thread is the main thread.
pub fn is_main_thread() -> bool {
    MAIN_THREAD_ID
        .get()
        .map(|id| *id == std::thread::current().id())
        .unwrap_or(false)
}

// =============================================================================
// Context-aware diagnostic helpers
// =============================================================================

/// Check frame context and emit appropriate diagnostic.
pub fn check_frame_context() {
    let ctx = DiagContext::capture();

    if !ctx.frame_active {
        if ctx.is_bevy {
            // Bevy-specific message
            super::emit::emit_with_context(
                &super::kind::FA101,
                &ctx.format(),
            );
        } else {
            // Generic message
            super::emit::emit_with_context(
                &super::kind::FA001,
                &ctx.format(),
            );
        }
    }
}

/// Check if we should warn about Bevy plugin.
pub fn check_bevy_plugin() {
    #[cfg(feature = "bevy")]
    {
        if !is_bevy_context() {
            super::emit::emit(&super::kind::FA101);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_capture() {
        let ctx = DiagContext::minimal();
        assert!(!ctx.is_bevy);
        assert!(!ctx.frame_active);
    }

    #[test]
    fn test_frame_counter() {
        reset_frame_counter();
        assert_eq!(frame_number(), 0);
        
        increment_frame();
        assert_eq!(frame_number(), 1);
        
        increment_frame();
        assert_eq!(frame_number(), 2);
        
        reset_frame_counter();
    }

    #[test]
    fn test_bevy_context() {
        set_bevy_context(false);
        assert!(!is_bevy_context());
        
        set_bevy_context(true);
        assert!(is_bevy_context());
        
        set_bevy_context(false);
    }

    #[test]
    fn test_context_format() {
        reset_frame_counter();
        set_bevy_context(false);
        
        let ctx = DiagContext::minimal();
        let formatted = ctx.format();
        
        assert!(formatted.contains("frame=0"));
    }
}
