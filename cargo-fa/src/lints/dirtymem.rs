//! Dirty memory lint pass (FA6xx).
//!
//! Detects:
//! - FA601: Frame allocation escapes scope
//! - FA602: Allocation in hot loop
//! - FA603: Missing frame boundaries
//! - FA604: Retention policy mismatch
//! - FA605: Discard policy but stored beyond frame

use crate::config::Config;
use crate::diagnostics::{self, Diagnostic, Location};
use crate::lints::{is_framealloc_call, FrameallocCall};
use crate::parser::span_to_location;
use std::path::Path;
use syn::visit::Visit;
use syn::spanned::Spanned;

pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = DirtyMemVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

struct DirtyMemVisitor<'a> {
    path: &'a Path,
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
    
    // Context tracking
    in_loop: bool,
    loop_depth: usize,
    current_loop_span: Option<proc_macro2::Span>,
    
    // Frame boundary tracking
    has_begin_frame: bool,
    has_end_frame: bool,
    in_main_loop: bool,
    
    // Allocation tracking for escape analysis
    frame_allocations: Vec<FrameAllocation>,
}

struct FrameAllocation {
    var_name: String,
    span: proc_macro2::Span,
    call_type: FrameallocCall,
}

impl<'a> DirtyMemVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        Self {
            path,
            config,
            diagnostics: Vec::new(),
            in_loop: false,
            loop_depth: 0,
            current_loop_span: None,
            has_begin_frame: false,
            has_end_frame: false,
            in_main_loop: false,
            frame_allocations: Vec::new(),
        }
    }
    
    fn check_allocation_in_loop(&mut self, call: FrameallocCall, span: proc_macro2::Span) {
        if self.in_loop && self.config.is_lint_enabled("FA602") {
            let loop_type = if self.loop_depth > 1 { "nested" } else { "tight" };
            
            self.diagnostics.push(diagnostics::fa602(
                span_to_location(span, self.path),
                call.alloc_type_str(),
                loop_type,
            ));
        }
    }
    
    fn check_frame_escape(&mut self, var_name: &str, assigned_to: &str, span: proc_macro2::Span) {
        // Check if this variable was a frame allocation
        if let Some(alloc) = self.frame_allocations.iter().find(|a| a.var_name == var_name) {
            if self.config.is_lint_enabled("FA601") {
                self.diagnostics.push(diagnostics::fa601(
                    span_to_location(span, self.path),
                    assigned_to,
                ));
            }
        }
    }
}

impl<'a> Visit<'a> for DirtyMemVisitor<'a> {
    fn visit_item_fn(&mut self, func: &'a syn::ItemFn) {
        // Check if this is main()
        let is_main = func.sig.ident == "main";
        
        if is_main {
            // Reset frame boundary tracking for main
            self.has_begin_frame = false;
            self.has_end_frame = false;
        }
        
        syn::visit::visit_item_fn(self, func);
        
        // After visiting main, check for missing frame boundaries
        if is_main && self.in_main_loop && !self.has_begin_frame && !self.has_end_frame {
            if self.config.is_lint_enabled("FA603") {
                self.diagnostics.push(diagnostics::fa603(
                    span_to_location(func.sig.ident.span(), self.path),
                ));
            }
        }
    }
    
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        match expr {
            // Track loop context
            syn::Expr::Loop(loop_expr) => {
                let was_in_loop = self.in_loop;
                let prev_depth = self.loop_depth;
                let prev_span = self.current_loop_span;
                
                self.in_loop = true;
                self.loop_depth += 1;
                self.current_loop_span = Some(loop_expr.span());
                self.in_main_loop = true;
                
                syn::visit::visit_expr_loop(self, loop_expr);
                
                self.in_loop = was_in_loop;
                self.loop_depth = prev_depth;
                self.current_loop_span = prev_span;
                return;
            }
            
            syn::Expr::While(while_expr) => {
                let was_in_loop = self.in_loop;
                let prev_depth = self.loop_depth;
                let prev_span = self.current_loop_span;
                
                self.in_loop = true;
                self.loop_depth += 1;
                self.current_loop_span = Some(while_expr.span());
                self.in_main_loop = true;
                
                syn::visit::visit_expr_while(self, while_expr);
                
                self.in_loop = was_in_loop;
                self.loop_depth = prev_depth;
                self.current_loop_span = prev_span;
                return;
            }
            
            syn::Expr::ForLoop(for_expr) => {
                let was_in_loop = self.in_loop;
                let prev_depth = self.loop_depth;
                let prev_span = self.current_loop_span;
                
                self.in_loop = true;
                self.loop_depth += 1;
                self.current_loop_span = Some(for_expr.span());
                self.in_main_loop = true;
                
                syn::visit::visit_expr_for_loop(self, for_expr);
                
                self.in_loop = was_in_loop;
                self.loop_depth = prev_depth;
                self.current_loop_span = prev_span;
                return;
            }
            
            // Check method calls
            syn::Expr::MethodCall(call) => {
                if let Some(fa_call) = is_framealloc_call(expr) {
                    // Track frame boundary calls
                    match fa_call {
                        FrameallocCall::BeginFrame => self.has_begin_frame = true,
                        FrameallocCall::EndFrame => self.has_end_frame = true,
                        _ => {}
                    }
                    
                    // Check for allocations in loops
                    if fa_call.is_any_allocation() {
                        self.check_allocation_in_loop(fa_call, call.span());
                    }
                }
            }
            
            // Check assignments for escape analysis
            syn::Expr::Assign(assign) => {
                // Check if RHS references a frame allocation variable
                if let syn::Expr::Path(path) = assign.right.as_ref() {
                    if let Some(ident) = path.path.get_ident() {
                        let var_name = ident.to_string();
                        
                        // Check if LHS is a field access (potential escape)
                        if let syn::Expr::Field(field) = assign.left.as_ref() {
                            self.check_frame_escape(
                                &var_name,
                                &format!("field assignment"),
                                assign.span(),
                            );
                        }
                    }
                }
            }
            
            _ => {}
        }
        
        syn::visit::visit_expr(self, expr);
    }
    
    fn visit_local(&mut self, local: &'a syn::Local) {
        // Track frame allocations bound to variables
        if let Some(init) = &local.init {
            if let Some(fa_call) = is_framealloc_call(&init.expr) {
                if fa_call.is_frame_allocation() {
                    if let syn::Pat::Ident(pat_ident) = &local.pat {
                        self.frame_allocations.push(FrameAllocation {
                            var_name: pat_ident.ident.to_string(),
                            span: local.span(),
                            call_type: fa_call,
                        });
                    }
                }
            }
        }
        
        syn::visit::visit_local(self, local);
    }
}
