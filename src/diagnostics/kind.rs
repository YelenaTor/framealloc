//! Diagnostic kinds and core types.
//!
//! Mirrors rustc's diagnostic levels for familiar UX.

/// Diagnostic code wrapper for type-safe code references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    /// Create a new diagnostic code.
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }
    
    /// Get the code string.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Diagnostic severity level (for behavior analysis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticLevel {
    /// Informational hint - not a problem, just a suggestion.
    Hint,
    /// Warning - probably suboptimal but not necessarily wrong.
    Warning,
    /// Error - definitely a problem that should be fixed.
    Error,
}

impl std::fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hint => write!(f, "hint"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// The severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// A hard error - something is definitely wrong.
    Error,
    /// A warning - something is probably wrong or suboptimal.
    Warning,
    /// Additional context about another diagnostic.
    Note,
    /// Actionable suggestion to fix the issue.
    Help,
}

impl DiagnosticKind {
    /// Get the display prefix for this kind.
    pub fn prefix(&self) -> &'static str {
        match self {
            DiagnosticKind::Error => "error",
            DiagnosticKind::Warning => "warning",
            DiagnosticKind::Note => "note",
            DiagnosticKind::Help => "help",
        }
    }

    /// Get the emoji for this kind (for build.rs style output).
    pub fn emoji(&self) -> &'static str {
        match self {
            DiagnosticKind::Error => "❌",
            DiagnosticKind::Warning => "⚠️",
            DiagnosticKind::Note => "ℹ️",
            DiagnosticKind::Help => "💡",
        }
    }
}

/// A diagnostic message with code, message, and optional context.
///
/// Diagnostic codes follow the pattern:
/// - `FA0xx` - Frame allocation issues
/// - `FA1xx` - Bevy integration issues
/// - `FA2xx` - Threading issues
/// - `FA3xx` - Budget/limit issues
/// - `FA4xx` - Handle/streaming issues
/// - `FA9xx` - Internal errors
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub kind: DiagnosticKind,
    /// Diagnostic code (e.g., "FA001").
    pub code: &'static str,
    /// Primary message.
    pub message: &'static str,
    /// Optional additional context.
    pub note: Option<&'static str>,
    /// Optional fix suggestion.
    pub help: Option<&'static str>,
}

impl Diagnostic {
    /// Create a new error diagnostic.
    pub const fn error(code: &'static str, message: &'static str) -> Self {
        Self {
            kind: DiagnosticKind::Error,
            code,
            message,
            note: None,
            help: None,
        }
    }

    /// Create a new warning diagnostic.
    pub const fn warning(code: &'static str, message: &'static str) -> Self {
        Self {
            kind: DiagnosticKind::Warning,
            code,
            message,
            note: None,
            help: None,
        }
    }

    /// Add a note to this diagnostic.
    pub const fn with_note(mut self, note: &'static str) -> Self {
        self.note = Some(note);
        self
    }

    /// Add a help message to this diagnostic.
    pub const fn with_help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }
}

// =============================================================================
// Predefined diagnostics (FA0xx - Frame allocation)
// =============================================================================

/// FA001: Frame allocation used outside an active frame.
pub const FA001: Diagnostic = Diagnostic::error(
    "FA001",
    "frame allocation used outside an active frame"
).with_note("this allocation was requested when no frame was active")
 .with_help("call alloc.begin_frame() before allocating, or use pool_alloc()/heap_alloc() for persistent data");

/// FA002: Frame memory escaped its scope.
pub const FA002: Diagnostic = Diagnostic::error(
    "FA002",
    "frame memory reference escaped frame scope"
).with_note("frame memory is invalidated at end_frame()")
 .with_help("copy data to persistent storage before end_frame(), or use pool_box()/heap_box()");

/// FA003: Frame arena exhausted.
pub const FA003: Diagnostic = Diagnostic::warning(
    "FA003",
    "frame arena exhausted, allocation failed"
).with_note("the frame arena ran out of space")
 .with_help("increase frame_arena_size in AllocConfig, or reduce per-frame allocations");

// =============================================================================
// Predefined diagnostics (FA1xx - Bevy integration)
// =============================================================================

/// FA101: Bevy plugin not registered.
pub const FA101: Diagnostic = Diagnostic::error(
    "FA101",
    "Bevy feature enabled but SmartAllocPlugin may not be registered"
).with_note("frame boundaries won't be managed automatically without the plugin")
 .with_help("add .add_plugins(framealloc::bevy::SmartAllocPlugin::default()) to your App");

/// FA102: Frame hooks not executed.
pub const FA102: Diagnostic = Diagnostic::warning(
    "FA102",
    "frame hooks have not been executed this frame"
).with_note("begin_frame()/end_frame() should be called each frame")
 .with_help("ensure SmartAllocPlugin is added, or call frame hooks manually");

// =============================================================================
// Predefined diagnostics (FA2xx - Threading)
// =============================================================================

/// FA201: Invalid cross-thread free.
pub const FA201: Diagnostic = Diagnostic::error(
    "FA201",
    "invalid cross-thread memory free"
).with_note("memory was freed from a different thread than it was allocated on")
 .with_help("use the deferred free queue for cross-thread frees, or ensure same-thread deallocation");

/// FA202: Thread-local state not initialized.
pub const FA202: Diagnostic = Diagnostic::warning(
    "FA202",
    "thread-local allocator state accessed before initialization"
).with_note("TLS is lazily initialized on first use")
 .with_help("this is usually fine, but may indicate unexpected thread usage");

// =============================================================================
// Predefined diagnostics (FA3xx - Budgets)
// =============================================================================

/// FA301: Allocation exceeds budget.
pub const FA301: Diagnostic = Diagnostic::warning(
    "FA301",
    "allocation exceeds memory budget"
).with_note("the allocation would push memory usage over the configured limit")
 .with_help("increase budget limits or reduce allocations");

/// FA302: Tag budget exceeded.
pub const FA302: Diagnostic = Diagnostic::warning(
    "FA302",
    "allocation exceeds tag-specific budget"
).with_note("this allocation tag has exceeded its hard limit")
 .with_help("check for memory leaks in this subsystem or increase the tag budget");

// =============================================================================
// Predefined diagnostics (FA4xx - Handles/Streaming)
// =============================================================================

/// FA401: Invalid handle access.
pub const FA401: Diagnostic = Diagnostic::error(
    "FA401",
    "attempted to access an invalid or freed handle"
).with_note("the handle's generation doesn't match, indicating it was freed")
 .with_help("ensure handles are not used after free, or check is_valid() first");

/// FA402: Streaming budget exhausted.
pub const FA402: Diagnostic = Diagnostic::warning(
    "FA402",
    "streaming allocator budget exhausted"
).with_note("no more streaming memory available and eviction failed")
 .with_help("increase streaming budget or free unused streaming allocations");

// =============================================================================
// Predefined diagnostics (FA9xx - Internal)
// =============================================================================

/// FA901: Internal allocator error.
pub const FA901: Diagnostic = Diagnostic::error(
    "FA901",
    "internal allocator error"
).with_note("this indicates a bug in framealloc")
 .with_help("please report this issue at the framealloc repository");
