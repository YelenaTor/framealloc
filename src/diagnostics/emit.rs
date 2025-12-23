//! Diagnostic emission backend.
//!
//! Handles outputting diagnostics to stderr, logs, or custom sinks.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use super::kind::{Diagnostic, DiagnosticKind};
use super::strict::should_panic;

/// Global flag to suppress diagnostic output (for testing).
static DIAGNOSTICS_SUPPRESSED: AtomicBool = AtomicBool::new(false);

/// Global flag to enable verbose diagnostics.
static VERBOSE_DIAGNOSTICS: AtomicBool = AtomicBool::new(false);

/// Suppress all diagnostic output.
pub fn suppress_diagnostics(suppress: bool) {
    DIAGNOSTICS_SUPPRESSED.store(suppress, Ordering::Relaxed);
}

/// Enable verbose diagnostic output.
pub fn set_verbose(verbose: bool) {
    VERBOSE_DIAGNOSTICS.store(verbose, Ordering::Relaxed);
}

/// Check if diagnostics are suppressed.
pub fn is_suppressed() -> bool {
    DIAGNOSTICS_SUPPRESSED.load(Ordering::Relaxed)
}

/// Emit a diagnostic to stderr.
///
/// In release builds without the `diagnostics` feature, this is a no-op.
/// In debug builds, this always emits.
/// In release builds with `diagnostics`, this emits based on configuration.
pub fn emit(diag: &Diagnostic) {
    // Check suppression first
    if is_suppressed() {
        return;
    }

    // Only emit in debug builds by default, unless diagnostics feature is on
    #[cfg(any(debug_assertions, feature = "diagnostics"))]
    {
        emit_to_stderr(diag);
    }

    // Check if we should panic (strict mode)
    if diag.kind == DiagnosticKind::Error && should_panic() {
        panic!(
            "[framealloc][{}] {}\nStrict mode enabled - errors are fatal.",
            diag.code, diag.message
        );
    }
}

/// Emit a diagnostic with additional runtime context.
pub fn emit_with_context(diag: &Diagnostic, context: &str) {
    if is_suppressed() {
        return;
    }

    #[cfg(any(debug_assertions, feature = "diagnostics"))]
    {
        emit_to_stderr_with_context(diag, context);
    }

    if diag.kind == DiagnosticKind::Error && should_panic() {
        panic!(
            "[framealloc][{}] {}\nContext: {}\nStrict mode enabled - errors are fatal.",
            diag.code, diag.message, context
        );
    }
}

/// Internal: emit to stderr.
#[cfg(any(debug_assertions, feature = "diagnostics"))]
fn emit_to_stderr(diag: &Diagnostic) {
    let mut stderr = std::io::stderr();
    let verbose = VERBOSE_DIAGNOSTICS.load(Ordering::Relaxed);

    // Main diagnostic line
    let _ = writeln!(
        stderr,
        "[framealloc][{}] {}: {}",
        diag.code,
        diag.kind.prefix(),
        diag.message
    );

    // Note (if present)
    if let Some(note) = diag.note {
        let _ = writeln!(stderr, "  note: {}", note);
    }

    // Help (if present)
    if let Some(help) = diag.help {
        let _ = writeln!(stderr, "  help: {}", help);
    }

    // Verbose: add stack trace hint
    if verbose && diag.kind == DiagnosticKind::Error {
        let _ = writeln!(stderr, "  hint: set RUST_BACKTRACE=1 for a backtrace");
    }

    // Blank line for readability
    let _ = writeln!(stderr);
}

/// Internal: emit to stderr with context.
#[cfg(any(debug_assertions, feature = "diagnostics"))]
fn emit_to_stderr_with_context(diag: &Diagnostic, context: &str) {
    let mut stderr = std::io::stderr();

    // Main diagnostic line
    let _ = writeln!(
        stderr,
        "[framealloc][{}] {}: {}",
        diag.code,
        diag.kind.prefix(),
        diag.message
    );

    // Context
    let _ = writeln!(stderr, "  context: {}", context);

    // Note (if present)
    if let Some(note) = diag.note {
        let _ = writeln!(stderr, "  note: {}", note);
    }

    // Help (if present)
    if let Some(help) = diag.help {
        let _ = writeln!(stderr, "  help: {}", help);
    }

    let _ = writeln!(stderr);
}

/// Emit a diagnostic using the log crate (if available).
#[cfg(feature = "log")]
pub fn emit_to_log(diag: &Diagnostic) {
    match diag.kind {
        DiagnosticKind::Error => {
            log::error!("[{}] {}", diag.code, diag.message);
        }
        DiagnosticKind::Warning => {
            log::warn!("[{}] {}", diag.code, diag.message);
        }
        DiagnosticKind::Note | DiagnosticKind::Help => {
            log::info!("[{}] {}", diag.code, diag.message);
        }
    }

    if let Some(note) = diag.note {
        log::info!("  note: {}", note);
    }
    if let Some(help) = diag.help {
        log::info!("  help: {}", help);
    }
}

/// A diagnostic sink trait for custom output.
pub trait DiagnosticSink: Send + Sync {
    /// Handle a diagnostic.
    fn emit(&self, diag: &Diagnostic);
}

/// A simple sink that collects diagnostics.
#[derive(Default)]
pub struct CollectingSink {
    diagnostics: std::sync::Mutex<Vec<Diagnostic>>,
}

impl CollectingSink {
    /// Create a new collecting sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all collected diagnostics.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.lock().unwrap().clone()
    }

    /// Clear collected diagnostics.
    pub fn clear(&self) {
        self.diagnostics.lock().unwrap().clear();
    }

    /// Check if any errors were collected.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .lock()
            .unwrap()
            .iter()
            .any(|d| d.kind == DiagnosticKind::Error)
    }
}

impl DiagnosticSink for CollectingSink {
    fn emit(&self, diag: &Diagnostic) {
        self.diagnostics.lock().unwrap().push(diag.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::kind::FA001;

    #[test]
    fn test_collecting_sink() {
        let sink = CollectingSink::new();
        sink.emit(&FA001);

        assert_eq!(sink.diagnostics().len(), 1);
        assert!(sink.has_errors());

        sink.clear();
        assert_eq!(sink.diagnostics().len(), 0);
    }

    #[test]
    fn test_suppression() {
        suppress_diagnostics(true);
        assert!(is_suppressed());
        suppress_diagnostics(false);
        assert!(!is_suppressed());
    }
}
