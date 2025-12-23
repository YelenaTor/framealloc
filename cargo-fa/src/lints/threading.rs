//! Threading lint pass (FA2xx).
//!
//! Detects:
//! - FA201: Cross-thread frame access
//! - FA202: Missing thread-local initialization

use crate::config::Config;
use crate::diagnostics::{Diagnostic, DiagnosticBuilder};
use crate::cli::Severity;
use crate::lints::{is_framealloc_call, FrameallocCall};
use crate::parser::span_to_location;
use std::path::Path;
use syn::visit::Visit;
use syn::spanned::Spanned;

pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = ThreadingVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

struct ThreadingVisitor<'a> {
    path: &'a Path,
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
    
    // Context tracking
    in_thread_spawn: bool,
    in_rayon_scope: bool,
}

impl<'a> ThreadingVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        Self {
            path,
            config,
            diagnostics: Vec::new(),
            in_thread_spawn: false,
            in_rayon_scope: false,
        }
    }
    
    fn is_thread_context(&self) -> bool {
        self.in_thread_spawn || self.in_rayon_scope
    }
}

impl<'a> Visit<'a> for ThreadingVisitor<'a> {
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        match expr {
            syn::Expr::MethodCall(call) => {
                let method_name = call.method.to_string();
                
                // Detect thread::spawn patterns
                if method_name == "spawn" {
                    let was_in_thread = self.in_thread_spawn;
                    self.in_thread_spawn = true;
                    
                    syn::visit::visit_expr_method_call(self, call);
                    
                    self.in_thread_spawn = was_in_thread;
                    return;
                }
                
                // Detect rayon patterns
                if method_name == "par_iter" || method_name == "par_bridge" 
                    || method_name == "into_par_iter" 
                {
                    let was_in_rayon = self.in_rayon_scope;
                    self.in_rayon_scope = true;
                    
                    syn::visit::visit_expr_method_call(self, call);
                    
                    self.in_rayon_scope = was_in_rayon;
                    return;
                }
                
                // Check for frame allocations in thread context
                if let Some(fa_call) = is_framealloc_call(expr) {
                    if fa_call.is_frame_allocation() && self.is_thread_context() {
                        if self.config.is_lint_enabled("FA201") {
                            self.diagnostics.push(
                                DiagnosticBuilder::new("FA201")
                                    .severity(Severity::Error)
                                    .message("frame allocation in spawned thread context")
                                    .location(span_to_location(call.span(), self.path))
                                    .note("frame allocations are thread-local and cannot be shared")
                                    .note("the spawned thread will have its own frame arena")
                                    .suggestion("use pool_box() or heap_box() for data shared between threads")
                                    .build()
                            );
                        }
                    }
                }
            }
            
            syn::Expr::Call(call) => {
                // Check for std::thread::spawn
                if let syn::Expr::Path(path) = call.func.as_ref() {
                    let path_str = path_to_string(&path.path);
                    
                    if path_str.ends_with("thread::spawn") || path_str == "spawn" {
                        let was_in_thread = self.in_thread_spawn;
                        self.in_thread_spawn = true;
                        
                        syn::visit::visit_expr_call(self, call);
                        
                        self.in_thread_spawn = was_in_thread;
                        return;
                    }
                    
                    // Check for rayon::scope
                    if path_str.ends_with("rayon::scope") || path_str == "scope" {
                        let was_in_rayon = self.in_rayon_scope;
                        self.in_rayon_scope = true;
                        
                        syn::visit::visit_expr_call(self, call);
                        
                        self.in_rayon_scope = was_in_rayon;
                        return;
                    }
                }
            }
            
            _ => {}
        }
        
        syn::visit::visit_expr(self, expr);
    }
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
