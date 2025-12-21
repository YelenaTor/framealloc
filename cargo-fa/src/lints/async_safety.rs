//! Async safety lint pass (FA7xx).
//!
//! Detects:
//! - FA701: Frame allocation in async function
//! - FA702: Frame allocation crosses await point
//! - FA703: FrameBox captured by closure/task

use crate::config::Config;
use crate::diagnostics::{self, Diagnostic, Location};
use crate::lints::{is_framealloc_call, FrameallocCall};
use crate::parser::span_to_location;
use std::path::Path;
use syn::visit::Visit;
use syn::spanned::Spanned;

pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = AsyncSafetyVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

struct AsyncSafetyVisitor<'a> {
    path: &'a Path,
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
    
    // Context tracking
    in_async_fn: bool,
    in_async_block: bool,
    
    // Track frame allocations and await points
    frame_alloc_before_await: Vec<FrameAllocInfo>,
    awaits_seen: Vec<proc_macro2::Span>,
}

struct FrameAllocInfo {
    var_name: Option<String>,
    span: proc_macro2::Span,
    call_type: FrameallocCall,
}

impl<'a> AsyncSafetyVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        Self {
            path,
            config,
            diagnostics: Vec::new(),
            in_async_fn: false,
            in_async_block: false,
            frame_alloc_before_await: Vec::new(),
            awaits_seen: Vec::new(),
        }
    }
    
    fn is_async_context(&self) -> bool {
        self.in_async_fn || self.in_async_block
    }
}

impl<'a> Visit<'a> for AsyncSafetyVisitor<'a> {
    fn visit_item_fn(&mut self, func: &'a syn::ItemFn) {
        let was_async = self.in_async_fn;
        self.in_async_fn = func.sig.asyncness.is_some();
        
        // Reset tracking for this function
        self.frame_alloc_before_await.clear();
        self.awaits_seen.clear();
        
        syn::visit::visit_item_fn(self, func);
        
        self.in_async_fn = was_async;
    }
    
    fn visit_impl_item_fn(&mut self, method: &'a syn::ImplItemFn) {
        let was_async = self.in_async_fn;
        self.in_async_fn = method.sig.asyncness.is_some();
        
        // Reset tracking for this function
        self.frame_alloc_before_await.clear();
        self.awaits_seen.clear();
        
        syn::visit::visit_impl_item_fn(self, method);
        
        self.in_async_fn = was_async;
    }
    
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        match expr {
            // Track async blocks
            syn::Expr::Async(async_expr) => {
                let was_async = self.in_async_block;
                self.in_async_block = true;
                
                syn::visit::visit_expr_async(self, async_expr);
                
                self.in_async_block = was_async;
                return;
            }
            
            // Check for frame allocations in async context
            syn::Expr::MethodCall(call) => {
                if let Some(fa_call) = is_framealloc_call(expr) {
                    if fa_call.is_frame_allocation() && self.is_async_context() {
                        // FA701: Frame allocation in async function
                        if self.config.is_lint_enabled("FA701") {
                            self.diagnostics.push(diagnostics::fa701(
                                span_to_location(call.span(), self.path),
                            ));
                        }
                        
                        // Track for await crossing analysis
                        self.frame_alloc_before_await.push(FrameAllocInfo {
                            var_name: None, // Will be set in visit_local
                            span: call.span(),
                            call_type: fa_call,
                        });
                    }
                }
            }
            
            // Track await points
            syn::Expr::Await(await_expr) => {
                self.awaits_seen.push(await_expr.span());
                
                // Check if any frame allocations came before this await
                // and might be used after (conservative: warn on all)
                if !self.frame_alloc_before_await.is_empty() && self.config.is_lint_enabled("FA702") {
                    for alloc in &self.frame_alloc_before_await {
                        self.diagnostics.push(diagnostics::fa702(
                            span_to_location(alloc.span, self.path),
                            span_to_location(await_expr.span(), self.path),
                        ));
                    }
                }
                
                syn::visit::visit_expr_await(self, await_expr);
                return;
            }
            
            // Check for closures capturing frame data
            syn::Expr::Closure(closure) => {
                // Check if closure might capture frame allocations
                // This is a conservative check - we look for any identifiers in the closure
                // that match known frame allocation variables
                
                if self.config.is_lint_enabled("FA703") {
                    // Check if this is a move closure (higher risk)
                    let is_move = closure.capture.is_some();
                    
                    // Look for spawn patterns
                    self.check_closure_for_frame_capture(closure, is_move);
                }
                
                syn::visit::visit_expr_closure(self, closure);
                return;
            }
            
            _ => {}
        }
        
        syn::visit::visit_expr(self, expr);
    }
    
    fn visit_local(&mut self, local: &'a syn::Local) {
        // Track variable names for frame allocations
        if let Some(init) = &local.init {
            if let Some(fa_call) = is_framealloc_call(&init.expr) {
                if fa_call.is_frame_allocation() {
                    if let syn::Pat::Ident(pat_ident) = &local.pat {
                        // Update the last frame allocation with the variable name
                        if let Some(last) = self.frame_alloc_before_await.last_mut() {
                            last.var_name = Some(pat_ident.ident.to_string());
                        }
                    }
                }
            }
        }
        
        syn::visit::visit_local(self, local);
    }
}

impl<'a> AsyncSafetyVisitor<'a> {
    fn check_closure_for_frame_capture(&mut self, closure: &syn::ExprClosure, is_move: bool) {
        // Simple heuristic: look for identifiers in the closure body that might be frame allocations
        // This is imprecise but catches common patterns
        
        struct IdentCollector {
            idents: Vec<String>,
        }
        
        impl<'ast> Visit<'ast> for IdentCollector {
            fn visit_ident(&mut self, ident: &'ast syn::Ident) {
                self.idents.push(ident.to_string());
            }
        }
        
        let mut collector = IdentCollector { idents: Vec::new() };
        collector.visit_expr(&closure.body);
        
        // Check if any collected idents match frame allocation variables
        for alloc in &self.frame_alloc_before_await {
            if let Some(ref var_name) = alloc.var_name {
                if collector.idents.contains(var_name) {
                    let capture_type = if is_move { "move closure" } else { "closure" };
                    self.diagnostics.push(diagnostics::fa703(
                        span_to_location(closure.span(), self.path),
                        capture_type,
                    ));
                    break; // One warning per closure is enough
                }
            }
        }
    }
}
