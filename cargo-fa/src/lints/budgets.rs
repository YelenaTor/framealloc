//! Budget lint pass (FA3xx).
//!
//! Detects:
//! - FA301: Unbounded allocations
//! - FA302: Missing budget guards

use crate::config::Config;
use crate::diagnostics::{Diagnostic, DiagnosticBuilder};
use crate::cli::Severity;
use crate::lints::{is_framealloc_call, FrameallocCall};
use crate::parser::span_to_location;
use std::path::Path;
use syn::visit::Visit;
use syn::spanned::Spanned;

pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = BudgetsVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

struct BudgetsVisitor<'a> {
    path: &'a Path,
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
    
    // Track loop allocations for unbounded detection
    loop_alloc_count: usize,
    in_loop: bool,
    loop_span: Option<proc_macro2::Span>,
}

impl<'a> BudgetsVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        Self {
            path,
            config,
            diagnostics: Vec::new(),
            loop_alloc_count: 0,
            in_loop: false,
            loop_span: None,
        }
    }
}

impl<'a> Visit<'a> for BudgetsVisitor<'a> {
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        match expr {
            // Track loops
            syn::Expr::Loop(loop_expr) => {
                let was_in_loop = self.in_loop;
                let prev_count = self.loop_alloc_count;
                let prev_span = self.loop_span;
                
                self.in_loop = true;
                self.loop_alloc_count = 0;
                self.loop_span = Some(loop_expr.span());
                
                syn::visit::visit_expr_loop(self, loop_expr);
                
                // Check if we exceeded threshold
                self.check_loop_allocations();
                
                self.in_loop = was_in_loop;
                self.loop_alloc_count = prev_count;
                self.loop_span = prev_span;
                return;
            }
            
            syn::Expr::While(while_expr) => {
                let was_in_loop = self.in_loop;
                let prev_count = self.loop_alloc_count;
                let prev_span = self.loop_span;
                
                self.in_loop = true;
                self.loop_alloc_count = 0;
                self.loop_span = Some(while_expr.span());
                
                syn::visit::visit_expr_while(self, while_expr);
                
                self.check_loop_allocations();
                
                self.in_loop = was_in_loop;
                self.loop_alloc_count = prev_count;
                self.loop_span = prev_span;
                return;
            }
            
            syn::Expr::ForLoop(for_expr) => {
                let was_in_loop = self.in_loop;
                let prev_count = self.loop_alloc_count;
                let prev_span = self.loop_span;
                
                self.in_loop = true;
                self.loop_alloc_count = 0;
                self.loop_span = Some(for_expr.span());
                
                syn::visit::visit_expr_for_loop(self, for_expr);
                
                self.check_loop_allocations();
                
                self.in_loop = was_in_loop;
                self.loop_alloc_count = prev_count;
                self.loop_span = prev_span;
                return;
            }
            
            // Count allocations in loops
            syn::Expr::MethodCall(_) => {
                if let Some(fa_call) = is_framealloc_call(expr) {
                    if fa_call.is_any_allocation() && self.in_loop {
                        self.loop_alloc_count += 1;
                    }
                }
            }
            
            _ => {}
        }
        
        syn::visit::visit_expr(self, expr);
    }
}

impl<'a> BudgetsVisitor<'a> {
    fn check_loop_allocations(&mut self) {
        // This is a static heuristic - we can't know actual iteration count
        // but we can warn about patterns that typically cause budget issues
        
        if self.loop_alloc_count > 0 && self.config.is_lint_enabled("FA301") {
            // For now, we just detect the pattern exists
            // More sophisticated analysis would try to determine loop bounds
            
            if let Some(span) = self.loop_span {
                // Only warn if there are multiple allocation calls in the loop
                // Single allocations might be intentional
                if self.loop_alloc_count >= 2 {
                    self.diagnostics.push(
                        DiagnosticBuilder::new("FA301")
                            .severity(Severity::Hint)
                            .message(format!(
                                "loop contains {} allocation calls - verify budget constraints",
                                self.loop_alloc_count
                            ))
                            .location(span_to_location(span, self.path))
                            .note("loops with allocations can exhaust memory budgets")
                            .suggestion("consider pre-allocating or using with_budget() guards")
                            .build()
                    );
                }
            }
        }
    }
}
