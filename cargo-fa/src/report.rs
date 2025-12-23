//! Report generation for cargo-fa.

use crate::cli::{OutputFormat, Severity};
use crate::diagnostics::Diagnostic;
use crate::output;

/// Analysis report containing all diagnostics.
#[derive(Debug, Default)]
pub struct Report {
    diagnostics: Vec<Diagnostic>,
    files_analyzed: usize,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add diagnostics from a file analysis
    pub fn add_diagnostics(&mut self, diags: Vec<Diagnostic>) {
        self.diagnostics.extend(diags);
        self.files_analyzed += 1;
    }
    
    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
    
    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Warning)
    }
    
    /// Count errors
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }
    
    /// Count warnings
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
    }
    
    /// Count hints
    pub fn hint_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Hint).count()
    }
    
    /// Print the report
    pub fn print(&self) {
        self.print_with_format(&OutputFormat::Terminal);
    }
    
    /// Print with specific format
    pub fn print_with_format(&self, format: &OutputFormat) {
        match format {
            OutputFormat::Sarif => {
                println!("{}", output::generate_sarif(&self.diagnostics));
            }
            OutputFormat::Junit => {
                println!("{}", output::generate_junit(&self.diagnostics));
            }
            OutputFormat::Checkstyle => {
                println!("{}", output::generate_checkstyle(&self.diagnostics));
            }
            OutputFormat::Json => {
                for diag in &self.diagnostics {
                    output::print_diagnostic(diag, format);
                }
                // Print summary as JSON
                println!("{}", serde_json::json!({
                    "summary": {
                        "files_analyzed": self.files_analyzed,
                        "errors": self.error_count(),
                        "warnings": self.warning_count(),
                        "hints": self.hint_count()
                    }
                }));
            }
            OutputFormat::Compact => {
                for diag in &self.diagnostics {
                    output::print_diagnostic(diag, format);
                }
            }
            OutputFormat::Terminal => {
                // Sort by severity (errors first)
                let mut sorted = self.diagnostics.clone();
                sorted.sort_by(|a, b| {
                    let severity_order = |s: &Severity| match s {
                        Severity::Error => 0,
                        Severity::Warning => 1,
                        Severity::Hint => 2,
                    };
                    severity_order(&a.severity).cmp(&severity_order(&b.severity))
                });
                
                for diag in &sorted {
                    output::print_diagnostic(diag, format);
                }
                
                println!();
                output::print_summary(
                    self.error_count(),
                    self.warning_count(),
                    self.hint_count()
                );
                println!("Analyzed {} files", self.files_analyzed);
            }
        }
    }
    
    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    
    /// Filter diagnostics by minimum severity
    pub fn filter_by_severity(&mut self, min: Severity) {
        self.diagnostics.retain(|d| d.severity >= min);
    }
}
