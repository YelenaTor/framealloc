//! Output formatting for cargo-fa diagnostics.

use crate::cli::{OutputFormat, Severity};
use crate::diagnostics::{Applicability, Diagnostic};
use colored::*;
use std::io::Write;

/// Format a header line
pub fn header(text: &str) -> String {
    format!("{} {}", "cargo-fa".cyan().bold(), text)
}

/// Print a diagnostic to the terminal
pub fn print_diagnostic(diag: &Diagnostic, format: &OutputFormat) {
    match format {
        OutputFormat::Terminal => print_terminal(diag),
        OutputFormat::Compact => print_compact(diag),
        OutputFormat::Json => print_json(diag),
        OutputFormat::Sarif => {}      // Handled at report level
        OutputFormat::Junit => {}      // Handled at report level  
        OutputFormat::Checkstyle => {} // Handled at report level
    }
}

fn print_terminal(diag: &Diagnostic) {
    let severity_str = match diag.severity {
        Severity::Error => "error".red().bold(),
        Severity::Warning => "warning".yellow().bold(),
        Severity::Hint => "hint".cyan().bold(),
    };
    
    // Main diagnostic line
    println!(
        "{}[{}]: {}",
        severity_str,
        diag.code.code.bold(),
        diag.message.bold()
    );
    
    // Location
    println!(
        "  {} {}:{}:{}",
        "-->".blue().bold(),
        diag.location.file.display(),
        diag.location.line,
        diag.location.column
    );
    
    // Show source context if possible
    if let Ok(source) = std::fs::read_to_string(&diag.location.file) {
        let lines: Vec<&str> = source.lines().collect();
        if diag.location.line > 0 && diag.location.line <= lines.len() {
            let line_num = diag.location.line;
            let line_str = format!("{}", line_num);
            let padding = " ".repeat(line_str.len());
            
            println!("   {} {}", padding, "|".blue().bold());
            println!(
                "   {} {} {}",
                line_num.to_string().blue().bold(),
                "|".blue().bold(),
                lines[line_num - 1]
            );
            
            // Underline the relevant part
            if diag.location.column > 0 {
                let underline_start = diag.location.column - 1;
                let underline_len = diag.location.end_column
                    .map(|e| e.saturating_sub(diag.location.column).max(1))
                    .unwrap_or(1);
                
                let underline = format!(
                    "{}{}",
                    " ".repeat(underline_start),
                    "^".repeat(underline_len)
                );
                
                let colored_underline = match diag.severity {
                    Severity::Error => underline.red().bold(),
                    Severity::Warning => underline.yellow().bold(),
                    Severity::Hint => underline.cyan().bold(),
                };
                
                println!(
                    "   {} {} {}",
                    padding,
                    "|".blue().bold(),
                    colored_underline
                );
            }
            
            println!("   {} {}", padding, "|".blue().bold());
        }
    }
    
    // Notes
    for note in &diag.notes {
        println!("   {} {}: {}", "=".blue().bold(), "note".bold(), note);
    }
    
    // Suggestion
    if let Some(ref suggestion) = diag.suggestion {
        let help_prefix = match suggestion.applicability {
            Applicability::MachineApplicable => "fix",
            _ => "help",
        };
        
        println!(
            "   {} {}: {}",
            "=".blue().bold(),
            help_prefix.green().bold(),
            suggestion.message
        );
        
        if let Some(ref replacement) = suggestion.replacement {
            println!("   {}  {}", " ".repeat(help_prefix.len()), replacement.green());
        }
    }
    
    // Related locations
    for related in &diag.related {
        println!(
            "   {} {}: {}",
            "=".blue().bold(),
            "related".bold(),
            related.message
        );
        println!(
            "      {} {}:{}:{}",
            "-->".blue(),
            related.location.file.display(),
            related.location.line,
            related.location.column
        );
    }
    
    // Documentation link
    println!(
        "   {} see: {}",
        "=".blue().bold(),
        format!("https://docs.rs/framealloc/diagnostics#{}", diag.code.code).dimmed()
    );
    
    println!();
}

fn print_compact(diag: &Diagnostic) {
    let severity = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Hint => "hint",
    };
    
    println!(
        "{}:{}:{}: {} [{}]: {}",
        diag.location.file.display(),
        diag.location.line,
        diag.location.column,
        severity,
        diag.code.code,
        diag.message
    );
}

fn print_json(diag: &Diagnostic) {
    if let Ok(json) = serde_json::to_string(diag) {
        println!("{}", json);
    }
}

/// Generate SARIF output for all diagnostics
pub fn generate_sarif(diagnostics: &[Diagnostic]) -> String {
    let results: Vec<serde_json::Value> = diagnostics
        .iter()
        .map(|d| {
            serde_json::json!({
                "ruleId": d.code.code,
                "level": match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Hint => "note",
                },
                "message": {
                    "text": d.message
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": d.location.file.to_string_lossy()
                        },
                        "region": {
                            "startLine": d.location.line,
                            "startColumn": d.location.column,
                            "endLine": d.location.end_line.unwrap_or(d.location.line),
                            "endColumn": d.location.end_column.unwrap_or(d.location.column + 1)
                        }
                    }
                }]
            })
        })
        .collect();
    
    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "cargo-fa",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://docs.rs/framealloc",
                    "rules": generate_rules()
                }
            },
            "results": results
        }]
    });
    
    serde_json::to_string_pretty(&sarif).unwrap_or_default()
}

fn generate_rules() -> Vec<serde_json::Value> {
    vec![
        rule("FA601", "frame-escape", "Frame allocation may escape frame scope"),
        rule("FA602", "loop-allocation", "Allocation in hot loop"),
        rule("FA603", "missing-frame-boundary", "Missing frame lifecycle calls"),
        rule("FA604", "retention-mismatch", "Retention policy mismatch"),
        rule("FA605", "discard-escape", "Discard policy but stored beyond frame"),
        rule("FA701", "async-frame", "Frame allocation in async function"),
        rule("FA702", "await-crossing", "Frame allocation crosses await point"),
        rule("FA703", "closure-capture", "FrameBox captured by closure/task"),
        rule("FA801", "tag-mismatch", "Allocation tag mismatch"),
        rule("FA802", "unknown-tag", "Unknown allocation tag"),
        rule("FA803", "cross-module", "Cross-module allocation"),
    ]
}

fn rule(id: &str, name: &str, description: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "shortDescription": { "text": description },
        "helpUri": format!("https://docs.rs/framealloc/diagnostics#{}", id)
    })
}

/// Print a summary line
pub fn print_summary(errors: usize, warnings: usize, hints: usize) {
    if errors == 0 && warnings == 0 && hints == 0 {
        println!("{}", "No issues found ✓".green().bold());
        return;
    }
    
    let mut parts = Vec::new();
    
    if errors > 0 {
        parts.push(format!("{} error{}", errors, if errors == 1 { "" } else { "s" }).red().bold().to_string());
    }
    if warnings > 0 {
        parts.push(format!("{} warning{}", warnings, if warnings == 1 { "" } else { "s" }).yellow().bold().to_string());
    }
    if hints > 0 {
        parts.push(format!("{} hint{}", hints, if hints == 1 { "" } else { "s" }).cyan().to_string());
    }
    
    println!("{}: {}", "Summary".bold(), parts.join(", "));
}

/// Generate JUnit XML output for test reporting systems
pub fn generate_junit(diagnostics: &[Diagnostic]) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="cargo-fa" tests=""#);
    xml.push_str(&diagnostics.len().to_string());
    xml.push_str(r#"" failures=""#);
    xml.push_str(&diagnostics.iter().filter(|d| d.severity == Severity::Error).count().to_string());
    xml.push_str(r#"">
  <testsuite name="framealloc-analysis">
"#);
    
    for diag in diagnostics {
        xml.push_str(&format!(
            r#"    <testcase name="{}" classname="{}">
"#,
            escape_xml(&diag.code.code),
            escape_xml(&diag.location.file.to_string_lossy())
        ));
        
        match diag.severity {
            Severity::Error => {
                xml.push_str(&format!(
                    r#"      <failure message="{}" type="error">{}</failure>
"#,
                    escape_xml(&diag.message),
                    escape_xml(&format!("{}:{}:{}", 
                        diag.location.file.display(),
                        diag.location.line,
                        diag.location.column
                    ))
                ));
            }
            Severity::Warning => {
                xml.push_str(&format!(
                    r#"      <failure message="{}" type="warning">{}</failure>
"#,
                    escape_xml(&diag.message),
                    escape_xml(&format!("{}:{}:{}", 
                        diag.location.file.display(),
                        diag.location.line,
                        diag.location.column
                    ))
                ));
            }
            Severity::Hint => {
                xml.push_str(&format!(
                    r#"      <system-out>{}: {}</system-out>
"#,
                    escape_xml(&diag.code.code),
                    escape_xml(&diag.message)
                ));
            }
        }
        
        xml.push_str("    </testcase>\n");
    }
    
    xml.push_str("  </testsuite>\n</testsuites>");
    xml
}

/// Generate Checkstyle XML output for Jenkins and legacy CI
pub fn generate_checkstyle(diagnostics: &[Diagnostic]) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<checkstyle version="4.3">
"#);
    
    // Group by file
    let mut by_file: std::collections::HashMap<String, Vec<&Diagnostic>> = std::collections::HashMap::new();
    for diag in diagnostics {
        let file = diag.location.file.to_string_lossy().to_string();
        by_file.entry(file).or_default().push(diag);
    }
    
    for (file, diags) in by_file {
        xml.push_str(&format!(r#"  <file name="{}">
"#, escape_xml(&file)));
        
        for diag in diags {
            let severity = match diag.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Hint => "info",
            };
            
            xml.push_str(&format!(
                r#"    <error line="{}" column="{}" severity="{}" message="{}" source="cargo-fa.{}"/>
"#,
                diag.location.line,
                diag.location.column,
                severity,
                escape_xml(&diag.message),
                diag.code.code
            ));
        }
        
        xml.push_str("  </file>\n");
    }
    
    xml.push_str("</checkstyle>");
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Generate full JSON output for fa-insight IDE integration (v0.7.0).
/// 
/// This produces the structured JSON format that fa-insight expects,
/// including diagnostics array and summary statistics.
pub fn generate_json_report(diagnostics: &[Diagnostic], files_analyzed: usize, duration_ms: u64) -> String {
    let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    let hints = diagnostics.iter().filter(|d| d.severity == Severity::Hint).count();
    
    let diag_json: Vec<serde_json::Value> = diagnostics
        .iter()
        .map(|d| {
            let mut obj = serde_json::json!({
                "code": {
                    "code": d.code.code,
                    "category": format!("{:?}", d.code.category)
                },
                "severity": match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Hint => "hint",
                },
                "message": d.message,
                "location": {
                    "file": d.location.file.to_string_lossy(),
                    "line": d.location.line,
                    "column": d.location.column,
                    "end_line": d.location.end_line,
                    "end_column": d.location.end_column
                },
                "notes": d.notes,
                "related": d.related.iter().map(|r| {
                    serde_json::json!({
                        "location": {
                            "file": r.location.file.to_string_lossy(),
                            "line": r.location.line,
                            "column": r.location.column
                        },
                        "message": r.message
                    })
                }).collect::<Vec<_>>()
            });
            
            if let Some(ref suggestion) = d.suggestion {
                obj["suggestion"] = serde_json::json!({
                    "message": suggestion.message,
                    "replacement": suggestion.replacement,
                    "applicability": format!("{:?}", suggestion.applicability)
                });
            }
            
            obj
        })
        .collect();
    
    let report = serde_json::json!({
        "diagnostics": diag_json,
        "summary": {
            "errors": errors,
            "warnings": warnings,
            "hints": hints
        },
        "files_analyzed": files_analyzed,
        "duration_ms": duration_ms
    });
    
    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}
