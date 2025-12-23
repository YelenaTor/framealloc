//! Lints for Rapier physics engine integration patterns.
//!
//! Detects common issues when integrating Rapier with framealloc.

use crate::config::Config;
use crate::cli::Severity;
use crate::diagnostics::{self, Diagnostic};
use crate::lints::LintPass;
use crate::parser::span_to_location;
use std::path::Path;
use syn::{visit::Visit, *};
use syn::spanned::Spanned;

/// Check for Rapier integration issues
pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = RapierVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

/// Lint pass for Rapier integration issues
pub struct RapierLint;

impl LintPass for RapierLint {
    fn name(&self) -> &'static str {
        "rapier"
    }

    fn check(&self, ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
        check(ast, path, config)
    }
}

/// Visitor to detect Rapier integration issues
struct RapierVisitor<'a> {
    diagnostics: Vec<Diagnostic>,
    path: &'a Path,
    config: &'a Config,
    in_rapier_context: bool,
    has_step_call: bool,
    has_broad_phase_usage: bool,
    has_query_filter_import: bool,
}

impl<'a> RapierVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        Self {
            diagnostics: Vec::new(),
            path,
            config,
            in_rapier_context: false,
            has_step_call: false,
            has_broad_phase_usage: false,
            has_query_filter_import: false,
        }
    }

    fn add_diagnostic(&mut self, code: &str, message: &str, span: proc_macro2::Span) {
        self.diagnostics.push(Diagnostic {
            code: diagnostics::DiagnosticCode::new(code),
            severity: Severity::Warning,
            message: message.to_string(),
            location: span_to_location(span, self.path),
            notes: vec![],
            suggestion: None,
            related: vec![],
        });
    }
    
    fn check_use_tree(&mut self, tree: &UseTree, path_so_far: &str) {
        match tree {
            UseTree::Path(path) => {
                let new_path = if path_so_far.is_empty() {
                    path.ident.to_string()
                } else {
                    format!("{}::{}", path_so_far, path.ident)
                };
                
                // Recursively check the rest of the path
                self.check_use_tree(&path.tree, &new_path);
            }
            
            UseTree::Group(group) => {
                // Check all items in the group
                for item in &group.items {
                    self.check_use_tree(item, path_so_far);
                }
            }
            
            UseTree::Name(name) => {
                let full_path = if path_so_far.is_empty() {
                    name.ident.to_string()
                } else {
                    format!("{}::{}", path_so_far, name.ident)
                };
                
                // Check for QueryFilter import
                if name.ident == "QueryFilter" {
                    self.has_query_filter_import = true;
                    // Check if it's imported from geometry (old location)
                    if path_so_far.contains("geometry") {
                        self.add_diagnostic(
                            "FA901",
                            "QueryFilter should be imported from rapier::pipeline, not rapier::geometry (Rapier 0.31)",
                            name.span(),
                        );
                    }
                }
                
                // Check for old BroadPhase import
                if name.ident == "BroadPhase" {
                    self.add_diagnostic(
                        "FA902",
                        "BroadPhase has been renamed to BroadPhaseBvh in Rapier 0.31",
                        name.span(),
                    );
                }
            }
            
            UseTree::Glob(glob) => {
                // Check the path leading to the glob
                if !path_so_far.is_empty() && path_so_far.contains("geometry") {
                    self.add_diagnostic(
                        "FA901",
                        "Use specific imports from rapier::pipeline instead of glob from rapier::geometry",
                        glob.span(),
                    );
                }
            }
            
            UseTree::Rename(rename) => {
                // Check the original name
                if rename.ident == "QueryFilter" {
                    self.has_query_filter_import = true;
                    if path_so_far.contains("geometry") {
                        self.add_diagnostic(
                            "FA901",
                            "QueryFilter should be imported from rapier::pipeline, not rapier::geometry (Rapier 0.31)",
                            rename.span(),
                        );
                    }
                } else if rename.ident == "BroadPhase" {
                    self.add_diagnostic(
                        "FA902",
                        "BroadPhase has been renamed to BroadPhaseBvh in Rapier 0.31",
                        rename.span(),
                    );
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for RapierVisitor<'ast> {
    fn visit_item_use(&mut self, i: &'ast ItemUse) {
        // Recursively check the use tree for QueryFilter and BroadPhase
        self.check_use_tree(&i.tree, "");
        visit::visit_item_use(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        // Check if we're in a Rapier-related struct
        if i.ident.to_string().contains("Physics") || 
           i.ident.to_string().contains("Rapier") {
            self.in_rapier_context = true;
        }
        visit::visit_item_struct(self, i);
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        // Check if implementing Rapier traits
        if let Some(path) = &i.trait_ {
            if let Some(segment) = path.1.segments.last() {
                if segment.ident == "PhysicsFrameAlloc" {
                    self.in_rapier_context = true;
                }
            }
        }
        visit::visit_item_impl(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();
        
        match method_name.as_str() {
            "step" => {
                // Check if it's PhysicsPipeline::step() without _with_events
                if let Expr::Path(path) = &*i.receiver {
                    if let Some(last) = path.path.segments.last() {
                        if last.ident == "physics_pipeline" || 
                           last.ident == "PhysicsPipeline" {
                            self.has_step_call = true;
                            self.add_diagnostic(
                                "FA903",
                                "Consider using step_with_events() instead of step() for frame-aware event collection",
                                i.method.span(),
                            );
                        }
                    }
                }
            }
            
            "cast_ray" | "intersect_ray" => {
                // Check if ray casting is done without prior step
                if !self.has_step_call {
                    self.add_diagnostic(
                        "FA904",
                        "Ray casting may not work correctly without calling step() first to update the broad phase",
                        i.method.span(),
                    );
                }
            }
            
            "frame_alloc_slice" => {
                self.add_diagnostic(
                    "FA905",
                    "frame_alloc_slice has been replaced with frame_alloc_batch + manual copying in Rapier 0.31",
                    i.method.span(),
                );
            }
            
            _ => {}
        }
        
        visit::visit_expr_method_call(self, i);
    }

    fn visit_macro(&mut self, i: &'ast Macro) {
        // Check for feature flags
        if i.path.segments.last().map(|s| s.ident.to_string()).as_deref() == Some("cfg") {
            // Parse the macro tokens to check for rapier feature
            let tokens = i.tokens.to_string();
            if tokens.contains("rapier") {
                self.in_rapier_context = true;
            }
        }
        visit::visit_macro(self, i);
    }
}
