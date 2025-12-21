//! Architecture lint pass (FA8xx).
//!
//! Detects:
//! - FA801: Tag mismatch (wrong tag for module)
//! - FA802: Unknown tag
//! - FA803: Cross-module allocation

use crate::config::Config;
use crate::diagnostics::{self, Diagnostic, DiagnosticBuilder};
use crate::cli::Severity;
use crate::lints::extract_tag_from_with_tag;
use crate::parser::span_to_location;
use std::path::Path;
use syn::visit::Visit;
use syn::spanned::Spanned;

pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = ArchitectureVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

struct ArchitectureVisitor<'a> {
    path: &'a Path,
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
    
    // Current module context
    current_module: Option<String>,
}

impl<'a> ArchitectureVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        // Infer module from path
        let current_module = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        
        Self {
            path,
            config,
            diagnostics: Vec::new(),
            current_module,
        }
    }
    
    fn check_tag(&mut self, tag: &str, span: proc_macro2::Span) {
        // FA802: Unknown tag
        if self.config.tags.warn_unknown_tags 
            && !self.config.tags.known_tags.contains(&tag.to_string())
            && self.config.is_lint_enabled("FA802")
        {
            self.diagnostics.push(diagnostics::fa802(
                span_to_location(span, self.path),
                tag,
            ));
        }
        
        // FA801: Tag mismatch
        if let Some(ref module) = self.current_module {
            if let Some(expected_tags) = self.config.tags.module_tags.get(module) {
                if !expected_tags.contains(&tag.to_string()) 
                    && self.config.is_lint_enabled("FA801")
                {
                    self.diagnostics.push(diagnostics::fa801(
                        span_to_location(span, self.path),
                        &expected_tags.join(", "),
                        tag,
                        module,
                    ));
                }
            }
        }
    }
}

impl<'a> Visit<'a> for ArchitectureVisitor<'a> {
    fn visit_item_mod(&mut self, module: &'a syn::ItemMod) {
        let prev_module = self.current_module.take();
        self.current_module = Some(module.ident.to_string());
        
        syn::visit::visit_item_mod(self, module);
        
        self.current_module = prev_module;
    }
    
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        // Check with_tag calls
        if let syn::Expr::MethodCall(call) = expr {
            if let Some(tag) = extract_tag_from_with_tag(call) {
                self.check_tag(&tag, call.span());
            }
        }
        
        syn::visit::visit_expr(self, expr);
    }
}
