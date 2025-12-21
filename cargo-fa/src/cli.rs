//! Command-line interface for cargo-fa.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cargo-fa",
    bin_name = "cargo",
    about = "Static analysis for framealloc - detect memory intent violations",
    version,
    after_help = "EXAMPLES:
    cargo fa --dirtymem           Check for dirty memory patterns
    cargo fa --threading          Check for threading issues  
    cargo fa --all                Run all checks
    cargo fa --format sarif       Output for GitHub Actions
    cargo fa explain FA601        Explain a diagnostic code
    cargo fa show src/physics.rs  Show diagnostics for a file
    
DOCUMENTATION:
    https://docs.rs/framealloc/diagnostics"
)]
pub struct Args {
    /// Cargo subcommand (fa)
    #[arg(hide = true)]
    pub subcommand: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to analyze (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,

    /// Check for dirty memory patterns (FA6xx)
    /// - Frame allocations escaping scope
    /// - Allocations in hot loops
    /// - Missing frame boundaries
    #[arg(long)]
    pub dirtymem: bool,

    /// Check for threading issues (FA2xx)
    /// - Cross-thread frame access
    /// - Missing thread-local initialization
    #[arg(long)]
    pub threading: bool,

    /// Check for budget violations (FA3xx)
    /// - Unbounded allocations
    /// - Missing budget guards
    #[arg(long)]
    pub budgets: bool,

    /// Check for async safety issues (FA7xx)
    /// - Frame allocations across await
    /// - Task-local misuse
    #[arg(long)]
    pub async_safety: bool,

    /// Check for architecture violations (FA8xx)
    /// - Tag mismatches
    /// - Module boundary violations
    #[arg(long)]
    pub architecture: bool,

    /// Run all checks (optimized order: fast checks first)
    #[arg(long, short = 'a')]
    pub all: bool,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "terminal")]
    pub format: OutputFormat,

    /// Treat specific lint as error (e.g., --deny FA601)
    #[arg(long, short = 'D', value_name = "CODE")]
    pub deny: Vec<String>,

    /// Allow specific lint (suppress warnings)
    #[arg(long, short = 'A', value_name = "CODE")]
    pub allow: Vec<String>,

    /// Treat warnings as errors
    #[arg(long, short = 'W')]
    pub deny_warnings: bool,

    /// Exclude paths from analysis (glob pattern)
    #[arg(long, short = 'e', value_name = "PATTERN")]
    pub exclude: Vec<String>,

    /// Show verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Configuration file path
    #[arg(long, default_value = ".fa.toml")]
    pub config: PathBuf,

    /// Minimum severity to report
    #[arg(long, value_enum, default_value = "hint")]
    pub min_severity: Severity,

    /// Specific lint codes to check (e.g., FA601,FA602)
    #[arg(long, value_delimiter = ',')]
    pub only: Option<Vec<String>>,

    /// Lint codes to skip
    #[arg(long, value_delimiter = ',')]
    pub skip: Option<Vec<String>>,

    /// Apply automatic fixes where possible
    #[arg(long)]
    pub fix: bool,

    /// Show what would be fixed without applying
    #[arg(long)]
    pub dry_run: bool,

    /// Fail fast: stop on first error
    #[arg(long)]
    pub fail_fast: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Explain a diagnostic code in detail
    Explain {
        /// Diagnostic code to explain (e.g., FA601)
        code: String,
    },
    
    /// Show diagnostics for a specific file with full context
    Show {
        /// File path to analyze
        file: PathBuf,
        
        /// Show suggested fixes inline
        #[arg(long)]
        fixes: bool,
    },
    
    /// Initialize a .fa.toml configuration file
    Init {
        /// Overwrite existing configuration
        #[arg(long)]
        force: bool,
    },
    
    /// List all available diagnostic codes
    List {
        /// Filter by category (dirtymem, threading, async, architecture)
        #[arg(long)]
        category: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output with colors
    #[default]
    Terminal,
    /// JSON output for programmatic consumption
    Json,
    /// SARIF format for GitHub Actions / VS Code
    Sarif,
    /// JUnit XML format for test reporting systems
    Junit,
    /// Checkstyle XML format for Jenkins and legacy CI
    Checkstyle,
    /// Compact one-line-per-issue format
    Compact,
}

#[derive(ValueEnum, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    /// Informational hints
    #[default]
    Hint,
    /// Warnings - probably wrong
    Warning,
    /// Errors - definitely wrong
    Error,
}

impl Args {
    /// Check if any check is enabled
    pub fn has_checks(&self) -> bool {
        self.all || self.dirtymem || self.threading || self.budgets 
            || self.async_safety || self.architecture
    }
}
