//! Lint passes for cargo-fa.
//!
//! Each module contains pattern detection logic for a category of issues.

pub mod dirtymem;
pub mod threading;
pub mod budgets;
pub mod async_safety;
pub mod architecture;
pub mod rapier;
pub mod gpu;

use crate::config::Config;
use crate::diagnostics::Diagnostic;
use std::path::Path;

/// Common trait for lint passes
pub trait LintPass {
    fn name(&self) -> &'static str;
    fn check(&self, ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic>;
}

/// Helper to check if an expression is a framealloc method call
pub fn is_framealloc_call(expr: &syn::Expr) -> Option<FrameallocCall> {
    if let syn::Expr::MethodCall(call) = expr {
        let method_name = call.method.to_string();
        
        match method_name.as_str() {
            "frame_alloc" => Some(FrameallocCall::FrameAlloc),
            "frame_box" => Some(FrameallocCall::FrameBox),
            "frame_vec" => Some(FrameallocCall::FrameVec),
            "frame_map" => Some(FrameallocCall::FrameMap),
            "frame_slice" => Some(FrameallocCall::FrameSlice),
            "frame_retained" => Some(FrameallocCall::FrameRetained),
            "frame_with_importance" => Some(FrameallocCall::FrameWithImportance),
            "pool_alloc" => Some(FrameallocCall::PoolAlloc),
            "pool_box" => Some(FrameallocCall::PoolBox),
            "heap_alloc" => Some(FrameallocCall::HeapAlloc),
            "heap_box" => Some(FrameallocCall::HeapBox),
            "with_tag" => Some(FrameallocCall::WithTag),
            "begin_frame" => Some(FrameallocCall::BeginFrame),
            "end_frame" => Some(FrameallocCall::EndFrame),
            "scratch_pool" => Some(FrameallocCall::ScratchPool),
            _ => None,
        }
    } else {
        None
    }
}

/// Types of framealloc calls we detect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameallocCall {
    FrameAlloc,
    FrameBox,
    FrameVec,
    FrameMap,
    FrameSlice,
    FrameRetained,
    FrameWithImportance,
    PoolAlloc,
    PoolBox,
    HeapAlloc,
    HeapBox,
    WithTag,
    BeginFrame,
    EndFrame,
    ScratchPool,
}

impl FrameallocCall {
    pub fn is_frame_allocation(&self) -> bool {
        matches!(
            self,
            Self::FrameAlloc
                | Self::FrameBox
                | Self::FrameVec
                | Self::FrameMap
                | Self::FrameSlice
                | Self::FrameRetained
                | Self::FrameWithImportance
        )
    }
    
    pub fn is_pool_allocation(&self) -> bool {
        matches!(self, Self::PoolAlloc | Self::PoolBox)
    }
    
    pub fn is_heap_allocation(&self) -> bool {
        matches!(self, Self::HeapAlloc | Self::HeapBox)
    }
    
    pub fn is_any_allocation(&self) -> bool {
        self.is_frame_allocation() || self.is_pool_allocation() || self.is_heap_allocation()
    }
    
    pub fn alloc_type_str(&self) -> &'static str {
        if self.is_frame_allocation() {
            "frame"
        } else if self.is_pool_allocation() {
            "pool"
        } else {
            "heap"
        }
    }
}

/// Check if we're inside a loop
pub fn is_in_loop(parents: &[&syn::Expr]) -> bool {
    parents.iter().any(|e| {
        matches!(e, syn::Expr::Loop(_) | syn::Expr::While(_) | syn::Expr::ForLoop(_))
    })
}

/// Get loop type string
pub fn loop_type_str(expr: &syn::Expr) -> &'static str {
    match expr {
        syn::Expr::Loop(_) => "loop",
        syn::Expr::While(_) => "while",
        syn::Expr::ForLoop(_) => "for",
        _ => "unknown",
    }
}

/// Extract tag string from with_tag call
pub fn extract_tag_from_with_tag(call: &syn::ExprMethodCall) -> Option<String> {
    if call.method == "with_tag" && !call.args.is_empty() {
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) = &call.args[0]
        {
            return Some(s.value());
        }
    }
    None
}
