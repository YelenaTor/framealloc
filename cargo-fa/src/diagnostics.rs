//! Diagnostic types and codes for cargo-fa.

use crate::cli::Severity;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A diagnostic message from static analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Diagnostic code (e.g., FA601)
    pub code: DiagnosticCode,
    
    /// Severity level
    pub severity: Severity,
    
    /// Primary message
    pub message: String,
    
    /// Source location
    pub location: Location,
    
    /// Additional context/notes
    pub notes: Vec<String>,
    
    /// Suggested fix
    pub suggestion: Option<Suggestion>,
    
    /// Related locations
    pub related: Vec<RelatedLocation>,
}

/// Source code location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

/// A related location for multi-span diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedLocation {
    pub location: Location,
    pub message: String,
}

/// A suggested fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub message: String,
    pub replacement: Option<String>,
    pub applicability: Applicability,
}

/// How confident we are in the suggestion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Applicability {
    /// Can be applied automatically
    MachineApplicable,
    /// Probably correct but needs review
    MaybeIncorrect,
    /// Needs human decision
    HasPlaceholders,
    /// Just informational
    Unspecified,
}

/// Diagnostic code with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCode {
    pub code: String,
    pub category: Category,
}

/// Diagnostic categories.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Category {
    /// FA6xx: Lifetime/escape issues
    Lifetime,
    /// FA7xx: Async safety
    AsyncSafety,
    /// FA8xx: Architecture violations
    Architecture,
    /// FA2xx: Threading
    Threading,
    /// FA3xx: Budgets
    Budgets,
}

impl DiagnosticCode {
    pub fn new(code: &str) -> Self {
        let category = match &code[2..3] {
            "6" => Category::Lifetime,
            "7" => Category::AsyncSafety,
            "8" => Category::Architecture,
            "2" => Category::Threading,
            "3" => Category::Budgets,
            _ => Category::Lifetime,
        };
        
        Self {
            code: code.to_string(),
            category,
        }
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}

// =============================================================================
// Predefined diagnostic codes (FA6xx - Lifetime/Escape)
// =============================================================================

/// FA601: Frame allocation escapes scope
pub fn fa601(location: Location, escaped_to: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA601"),
        severity: Severity::Warning,
        message: "frame allocation may escape frame scope".to_string(),
        location,
        notes: vec![
            format!("allocation appears to be stored in: {}", escaped_to),
            "frame allocations are invalidated at end_frame()".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "consider using pool_box() or heap_box() for data that outlives the frame".to_string(),
            replacement: None,
            applicability: Applicability::HasPlaceholders,
        }),
        related: vec![],
    }
}

/// FA602: Allocation in hot loop
pub fn fa602(location: Location, alloc_type: &str, loop_type: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA602"),
        severity: Severity::Warning,
        message: format!("{} allocation inside {} loop", alloc_type, loop_type),
        location,
        notes: vec![
            "allocations in tight loops can cause performance issues".to_string(),
            "consider pre-allocating or using frame_vec()".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "move allocation outside loop or use a pre-allocated buffer".to_string(),
            replacement: None,
            applicability: Applicability::HasPlaceholders,
        }),
        related: vec![],
    }
}

/// FA603: Missing frame boundaries
pub fn fa603(location: Location) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA603"),
        severity: Severity::Warning,
        message: "frame-structured loop without frame lifecycle calls".to_string(),
        location,
        notes: vec![
            "detected a main loop pattern without begin_frame()/end_frame()".to_string(),
            "frame allocations may accumulate indefinitely".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "add alloc.begin_frame() at loop start and alloc.end_frame() at loop end".to_string(),
            replacement: None,
            applicability: Applicability::HasPlaceholders,
        }),
        related: vec![],
    }
}

/// FA604: Retention policy mismatch
pub fn fa604(location: Location, policy: &str, actual_usage: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA604"),
        severity: Severity::Hint,
        message: format!("retention policy '{}' may not match usage pattern", policy),
        location,
        notes: vec![
            format!("observed usage: {}", actual_usage),
        ],
        suggestion: Some(Suggestion {
            message: "review retention policy choice".to_string(),
            replacement: None,
            applicability: Applicability::Unspecified,
        }),
        related: vec![],
    }
}

/// FA605: Discard policy but stored beyond frame
pub fn fa605(location: Location) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA605"),
        severity: Severity::Warning,
        message: "allocation with Discard policy stored beyond frame scope".to_string(),
        location,
        notes: vec![
            "RetentionPolicy::Discard means data is lost at frame end".to_string(),
            "but this allocation appears to be stored in a persistent structure".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "use PromoteToPool or PromoteToHeap if data needs to persist".to_string(),
            replacement: None,
            applicability: Applicability::HasPlaceholders,
        }),
        related: vec![],
    }
}

// =============================================================================
// Predefined diagnostic codes (FA7xx - Async Safety)
// =============================================================================

/// FA701: Frame allocation in async function
pub fn fa701(location: Location) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA701"),
        severity: Severity::Error,
        message: "frame allocation in async function".to_string(),
        location,
        notes: vec![
            "async functions may suspend across frame boundaries".to_string(),
            "frame allocations become invalid after end_frame()".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "use pool_box() or heap_box() for data in async contexts".to_string(),
            replacement: None,
            applicability: Applicability::MaybeIncorrect,
        }),
        related: vec![],
    }
}

/// FA702: Frame allocation crosses await point
pub fn fa702(location: Location, await_location: Location) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA702"),
        severity: Severity::Error,
        message: "frame allocation used across await point".to_string(),
        location,
        notes: vec![
            "the allocation is created before an await".to_string(),
            "and used after the await completes".to_string(),
            "frames may have been reset during the await".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "complete frame work before awaiting, or use persistent allocation".to_string(),
            replacement: None,
            applicability: Applicability::HasPlaceholders,
        }),
        related: vec![RelatedLocation {
            location: await_location,
            message: "await point here".to_string(),
        }],
    }
}

/// FA703: FrameBox captured by closure/task
pub fn fa703(location: Location, capture_type: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA703"),
        severity: Severity::Error,
        message: format!("FrameBox captured by {}", capture_type),
        location,
        notes: vec![
            format!("{} may outlive the current frame", capture_type),
            "FrameBox becomes invalid after end_frame()".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: "use PoolBox or HeapBox for data captured by closures/tasks".to_string(),
            replacement: None,
            applicability: Applicability::MaybeIncorrect,
        }),
        related: vec![],
    }
}

// =============================================================================
// Predefined diagnostic codes (FA8xx - Architecture)
// =============================================================================

/// FA801: Tag mismatch
pub fn fa801(location: Location, expected_tag: &str, actual_tag: &str, module: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA801"),
        severity: Severity::Warning,
        message: format!("allocation tag '{}' unexpected in module '{}'", actual_tag, module),
        location,
        notes: vec![
            format!("expected tags for this module: {}", expected_tag),
            "tag mismatches may indicate architectural confusion".to_string(),
        ],
        suggestion: Some(Suggestion {
            message: format!("use tag '{}' or move allocation to appropriate module", expected_tag),
            replacement: None,
            applicability: Applicability::HasPlaceholders,
        }),
        related: vec![],
    }
}

/// FA802: Unknown tag
pub fn fa802(location: Location, tag: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA802"),
        severity: Severity::Hint,
        message: format!("unknown allocation tag '{}'", tag),
        location,
        notes: vec![
            "this tag is not in the known_tags list in .fa.toml".to_string(),
            "consider adding it or using an existing tag".to_string(),
        ],
        suggestion: None,
        related: vec![],
    }
}

/// FA803: Cross-module allocation
pub fn fa803(location: Location, from_module: &str, to_module: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::new("FA803"),
        severity: Severity::Warning,
        message: format!("allocation intent crosses module boundary: {} -> {}", from_module, to_module),
        location,
        notes: vec![
            "allocations typically should stay within their module's concerns".to_string(),
        ],
        suggestion: None,
        related: vec![],
    }
}

impl Diagnostic {
    /// Create a diagnostic builder
    pub fn builder(code: &str) -> DiagnosticBuilder {
        DiagnosticBuilder::new(code)
    }
}

/// Builder for constructing diagnostics
pub struct DiagnosticBuilder {
    code: DiagnosticCode,
    severity: Severity,
    message: Option<String>,
    location: Option<Location>,
    notes: Vec<String>,
    suggestion: Option<Suggestion>,
    related: Vec<RelatedLocation>,
}

impl DiagnosticBuilder {
    pub fn new(code: &str) -> Self {
        Self {
            code: DiagnosticCode::new(code),
            severity: Severity::Warning,
            message: None,
            location: None,
            notes: Vec::new(),
            suggestion: None,
            related: Vec::new(),
        }
    }
    
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
    
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }
    
    pub fn location(mut self, loc: Location) -> Self {
        self.location = Some(loc);
        self
    }
    
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
    
    pub fn suggestion(mut self, msg: impl Into<String>) -> Self {
        self.suggestion = Some(Suggestion {
            message: msg.into(),
            replacement: None,
            applicability: Applicability::Unspecified,
        });
        self
    }
    
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            code: self.code,
            severity: self.severity,
            message: self.message.unwrap_or_default(),
            location: self.location.unwrap_or(Location {
                file: PathBuf::new(),
                line: 0,
                column: 0,
                end_line: None,
                end_column: None,
            }),
            notes: self.notes,
            suggestion: self.suggestion,
            related: self.related,
        }
    }
}
