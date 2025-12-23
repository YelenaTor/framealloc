//! cargo-fa: Static analysis tool for framealloc
//!
//! Detects memory intent violations before runtime by analyzing source code
//! for patterns that compile but violate frame allocation principles.
//!
//! # Usage
//!
//! ```bash
//! # Check for "dirty memory" patterns
//! cargo fa --dirtymem
//!
//! # Check threading issues
//! cargo fa --threading
//!
//! # Check everything (optimized order)
//! cargo fa --all
//!
//! # Output formats for CI
//! cargo fa --all --format sarif      # GitHub Actions
//! cargo fa --all --format junit      # Test reporters
//! cargo fa --all --format checkstyle # Jenkins
//!
//! # Filtering
//! cargo fa --all --deny FA701 --allow FA602
//! cargo fa --all --exclude "**/tests/**"
//!
//! # Subcommands
//! cargo fa explain FA601             # Detailed explanation
//! cargo fa show src/physics.rs       # Single file analysis
//! cargo fa list                      # List all codes
//! cargo fa init                      # Create .fa.toml
//! ```

mod cli;
mod config;
mod diagnostics;
mod explain;
mod lints;
mod output;
mod parser;
mod report;

use anyhow::Result;
use cli::{Args, Command, OutputFormat};
use clap::Parser;

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Handle cargo subcommand invocation (cargo fa → cargo-fa fa)
    let args = if args.subcommand.is_some() {
        args
    } else {
        Args::parse_from(std::env::args())
    };
    
    // Handle subcommands first
    if let Some(ref cmd) = args.command {
        return handle_subcommand(cmd, &args);
    }
    
    // Default: run analysis
    let analyzer = Analyzer::new(args)?;
    let report = analyzer.run()?;
    
    report.print_with_format(&analyzer.args.format);
    
    // Exit codes
    if report.has_errors() {
        std::process::exit(1);
    } else if report.has_warnings() && analyzer.args.deny_warnings {
        std::process::exit(1);
    } else if analyzer.has_denied_issues(&report) {
        std::process::exit(1);
    }
    
    Ok(())
}

fn handle_subcommand(cmd: &Command, args: &Args) -> Result<()> {
    match cmd {
        Command::Explain { code } => {
            if let Some(explanation) = explain::get_explanation(code) {
                explain::print_explanation(&explanation);
            } else {
                eprintln!("Unknown diagnostic code: {}", code);
                eprintln!("Run `cargo fa list` to see all available codes.");
                std::process::exit(1);
            }
        }
        
        Command::Show { file, fixes } => {
            let config = config::Config::load(args)?;
            
            if !file.exists() {
                eprintln!("File not found: {}", file.display());
                std::process::exit(1);
            }
            
            println!("{}", output::header(&format!("Analyzing {}", file.display())));
            
            match parser::parse_file(file) {
                Ok(ast) => {
                    let mut all_diags = Vec::new();
                    all_diags.extend(lints::dirtymem::check(&ast, file, &config));
                    all_diags.extend(lints::threading::check(&ast, file, &config));
                    all_diags.extend(lints::budgets::check(&ast, file, &config));
                    all_diags.extend(lints::async_safety::check(&ast, file, &config));
                    all_diags.extend(lints::gpu::check(&ast, file, &config));
                    all_diags.extend(lints::architecture::check(&ast, file, &config));
                    all_diags.extend(lints::rapier::check(&ast, file, &config));
                    
                    if all_diags.is_empty() {
                        println!("No issues found in {}.", file.display());
                    } else {
                        for diag in &all_diags {
                            output::print_diagnostic(diag, &OutputFormat::Terminal);
                        }
                        output::print_summary(
                            all_diags.iter().filter(|d| d.severity == cli::Severity::Error).count(),
                            all_diags.iter().filter(|d| d.severity == cli::Severity::Warning).count(),
                            all_diags.iter().filter(|d| d.severity == cli::Severity::Hint).count(),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse {}: {}", file.display(), e);
                    std::process::exit(1);
                }
            }
        }
        
        Command::Init { force } => {
            let config_path = args.path.join(".fa.toml");
            
            if config_path.exists() && !force {
                eprintln!(".fa.toml already exists. Use --force to overwrite.");
                std::process::exit(1);
            }
            
            std::fs::write(&config_path, config::generate_default_config())?;
            println!("Created {}", config_path.display());
        }
        
        Command::List { category } => {
            explain::list_all_codes(category.as_deref());
        }
    }
    
    Ok(())
}

struct Analyzer {
    args: Args,
    config: config::Config,
}

impl Analyzer {
    fn new(args: Args) -> Result<Self> {
        let config = config::Config::load(&args)?;
        Ok(Self { args, config })
    }
    
    fn run(&self) -> Result<report::Report> {
        let mut report = report::Report::new();
        
        // Find all Rust source files
        let mut source_files = parser::find_rust_files(&self.args.path)?;
        
        // Apply exclude patterns
        if !self.args.exclude.is_empty() {
            source_files.retain(|path| {
                let path_str = path.to_string_lossy();
                !self.args.exclude.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(&path_str))
                        .unwrap_or(false)
                })
            });
        }
        
        // Quiet mode for non-terminal formats
        let quiet = !matches!(self.args.format, OutputFormat::Terminal);
        
        if !quiet {
            println!(
                "{}",
                output::header(&format!("Analyzing {} files", source_files.len()))
            );
        }
        
        for file_path in &source_files {
            if self.args.verbose {
                println!("  Checking: {}", file_path.display());
            }
            
            match parser::parse_file(file_path) {
                Ok(ast) => {
                    let mut file_diagnostics = self.analyze_file(&ast, file_path);
                    
                    // Apply --allow filter
                    file_diagnostics.retain(|d| {
                        !self.args.allow.contains(&d.code.code)
                    });
                    
                    // Apply --deny (upgrade to error)
                    for diag in &mut file_diagnostics {
                        if self.args.deny.contains(&diag.code.code) {
                            diag.severity = cli::Severity::Error;
                        }
                    }
                    
                    // Apply --only filter
                    if let Some(ref only) = self.args.only {
                        file_diagnostics.retain(|d| only.contains(&d.code.code));
                    }
                    
                    // Apply --skip filter  
                    if let Some(ref skip) = self.args.skip {
                        file_diagnostics.retain(|d| !skip.contains(&d.code.code));
                    }
                    
                    // Apply min_severity filter
                    file_diagnostics.retain(|d| d.severity >= self.args.min_severity);
                    
                    report.add_diagnostics(file_diagnostics);
                    
                    // Fail fast on first error
                    if self.args.fail_fast && report.has_errors() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("ERROR: Failed to parse {}: {}", file_path.display(), e);
                    std::process::exit(1);
                }
            }
        }
        
        Ok(report)
    }
    
    fn analyze_file(
        &self,
        ast: &syn::File,
        path: &std::path::Path,
    ) -> Vec<diagnostics::Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Run checks in optimized order (fast checks first for --all)
        // This allows fail-fast to catch easy issues before expensive analysis
        
        // 1. Architecture (fastest - just tag checking)
        if self.args.architecture || self.args.all {
            diagnostics.extend(lints::architecture::check(ast, path, &self.config));
        }
        
        // 2. Dirtymem (fast AST traversal)
        if self.args.dirtymem || self.args.all {
            diagnostics.extend(lints::dirtymem::check(ast, path, &self.config));
        }
        
        // 3. Budgets (simple loop detection)
        if self.args.budgets || self.args.all {
            diagnostics.extend(lints::budgets::check(ast, path, &self.config));
        }
        
        // 4. Async safety (moderate - tracks across await)
        if self.args.async_safety || self.args.all {
            diagnostics.extend(lints::async_safety::check(ast, path, &self.config));
        }
        
        // 5. GPU memory safety (checks for staging buffer leaks, transfer issues)
        if self.args.gpu || self.args.all {
            diagnostics.extend(lints::gpu::check(ast, path, &self.config));
        }
        
        // 6. Threading (most complex - control flow analysis)
        if self.args.threading || self.args.all {
            diagnostics.extend(lints::threading::check(ast, path, &self.config));
        }
        
        // 6. Rapier integration (physics engine specific checks)
        if self.args.all {
            diagnostics.extend(lints::rapier::check(ast, path, &self.config));
        }
        
        diagnostics
    }
    
    /// Check if any diagnostics match --deny codes
    fn has_denied_issues(&self, report: &report::Report) -> bool {
        if self.args.deny.is_empty() {
            return false;
        }
        report.diagnostics().iter().any(|d| {
            self.args.deny.contains(&d.code.code)
        })
    }
}
