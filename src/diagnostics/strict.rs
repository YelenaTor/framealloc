//! Strict mode handling for diagnostics.
//!
//! Allows configuration of how diagnostics are treated:
//! - Warn: Just emit the diagnostic
//! - Panic: Emit and then panic (useful for CI)

use std::sync::atomic::{AtomicU8, Ordering};

/// Strict mode behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StrictMode {
    /// Just warn, don't panic.
    Warn = 0,
    /// Panic on errors.
    PanicOnError = 1,
    /// Panic on errors and warnings.
    PanicOnWarning = 2,
}

impl From<u8> for StrictMode {
    fn from(val: u8) -> Self {
        match val {
            0 => StrictMode::Warn,
            1 => StrictMode::PanicOnError,
            2 => StrictMode::PanicOnWarning,
            _ => StrictMode::Warn,
        }
    }
}

/// Global strict mode setting.
static STRICT_MODE: AtomicU8 = AtomicU8::new(0);

/// Set the strict mode.
pub fn set_strict_mode(mode: StrictMode) {
    STRICT_MODE.store(mode as u8, Ordering::Relaxed);
}

/// Get the current strict mode.
pub fn strict_mode() -> StrictMode {
    StrictMode::from(STRICT_MODE.load(Ordering::Relaxed))
}

/// Check if we should panic for the current diagnostic level.
pub fn should_panic() -> bool {
    matches!(strict_mode(), StrictMode::PanicOnError | StrictMode::PanicOnWarning)
}

/// Check if we should panic for warnings.
pub fn should_panic_on_warning() -> bool {
    matches!(strict_mode(), StrictMode::PanicOnWarning)
}

/// RAII guard for temporarily setting strict mode.
pub struct StrictModeGuard {
    previous: StrictMode,
}

impl StrictModeGuard {
    /// Create a new guard that sets strict mode.
    pub fn new(mode: StrictMode) -> Self {
        let previous = strict_mode();
        set_strict_mode(mode);
        Self { previous }
    }

    /// Create a guard that enables panic-on-error.
    pub fn panic_on_error() -> Self {
        Self::new(StrictMode::PanicOnError)
    }

    /// Create a guard that enables panic-on-warning.
    pub fn panic_on_warning() -> Self {
        Self::new(StrictMode::PanicOnWarning)
    }
}

impl Drop for StrictModeGuard {
    fn drop(&mut self) {
        set_strict_mode(self.previous);
    }
}

/// Initialize strict mode from environment variable.
///
/// Checks `FRAMEALLOC_STRICT` environment variable:
/// - "0" or "warn" -> Warn
/// - "1" or "error" -> PanicOnError
/// - "2" or "warning" -> PanicOnWarning
pub fn init_from_env() {
    if let Ok(val) = std::env::var("FRAMEALLOC_STRICT") {
        let mode = match val.to_lowercase().as_str() {
            "0" | "warn" | "false" => StrictMode::Warn,
            "1" | "error" | "true" => StrictMode::PanicOnError,
            "2" | "warning" | "all" => StrictMode::PanicOnWarning,
            _ => StrictMode::Warn,
        };
        set_strict_mode(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_mode_default() {
        // Reset to default
        set_strict_mode(StrictMode::Warn);
        assert_eq!(strict_mode(), StrictMode::Warn);
        assert!(!should_panic());
    }

    #[test]
    fn test_strict_mode_panic_on_error() {
        let _guard = StrictModeGuard::new(StrictMode::PanicOnError);
        assert_eq!(strict_mode(), StrictMode::PanicOnError);
        assert!(should_panic());
        assert!(!should_panic_on_warning());
    }

    #[test]
    fn test_strict_mode_guard() {
        set_strict_mode(StrictMode::Warn);
        
        {
            let _guard = StrictModeGuard::panic_on_error();
            assert_eq!(strict_mode(), StrictMode::PanicOnError);
        }
        
        // Guard dropped, should be back to Warn
        assert_eq!(strict_mode(), StrictMode::Warn);
    }
}
