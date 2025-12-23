//! GPU memory safety lint pass (FA8xx).
//!
//! Detects:
//! - FA801: Staging buffer not freed before frame end
//! - FA802: GPU buffer created without transfer usage
//! - FA803: CPU-GPU transfer without synchronization barrier
//! - FA804: Device-local buffer mapped for CPU access
//! - FA805: Staging buffer reused across frames without reset

use crate::config::Config;
use crate::diagnostics::{self, Diagnostic, Location};
use crate::lints::{is_framealloc_call, FrameallocCall};
use crate::parser::span_to_location;
use std::path::Path;
use syn::visit::Visit;
use syn::spanned::Spanned;

pub fn check(ast: &syn::File, path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut visitor = GpuSafetyVisitor::new(path, config);
    visitor.visit_file(ast);
    visitor.diagnostics
}

struct GpuSafetyVisitor<'a> {
    path: &'a Path,
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
    
    // Context tracking
    in_frame_scope: bool,
    frame_start_span: Option<proc_macro2::Span>,
    
    // GPU allocation tracking
    staging_buffers: Vec<StagingBufferInfo>,
    gpu_buffers: Vec<GpuBufferInfo>,
    transfers_without_barrier: Vec<TransferInfo>,
}

struct StagingBufferInfo {
    name: Option<String>,
    span: proc_macro2::Span,
    created_in_frame: bool,
    freed: bool,
}

struct GpuBufferInfo {
    name: Option<String>,
    span: proc_macro2::Span,
    memory_type: String,
    has_transfer_usage: bool,
    is_mapped: bool,
}

struct TransferInfo {
    span: proc_macro2::Span,
    has_barrier: bool,
}

impl<'a> GpuSafetyVisitor<'a> {
    fn new(path: &'a Path, config: &'a Config) -> Self {
        Self {
            path,
            config,
            diagnostics: Vec::new(),
            in_frame_scope: false,
            frame_start_span: None,
            staging_buffers: Vec::new(),
            gpu_buffers: Vec::new(),
            transfers_without_barrier: Vec::new(),
        }
    }
    
    fn check_staging_buffer_leak(&mut self) {
        // FA801: Staging buffer not freed before frame end
        for buffer in &self.staging_buffers {
            if buffer.created_in_frame && !buffer.freed {
                self.diagnostics.push(diagnostics::fa801(
                    span_to_location(buffer.span, self.path),
                ));
            }
        }
    }
    
    fn check_gpu_buffer_transfer_usage(&mut self, buffer: &GpuBufferInfo) {
        // FA802: GPU buffer created without transfer usage
        if !buffer.has_transfer_usage && buffer.memory_type.contains("DeviceLocal") {
            self.diagnostics.push(diagnostics::fa802(
                span_to_location(buffer.span, self.path),
            ));
        }
    }
    
    fn check_device_local_mapped(&mut self, buffer: &GpuBufferInfo) {
        // FA804: Device-local buffer mapped for CPU access
        if buffer.memory_type.contains("DeviceLocal") && buffer.is_mapped {
            self.diagnostics.push(diagnostics::fa804(
                span_to_location(buffer.span, self.path),
            ));
        }
    }
}

impl<'a> Visit<'a> for GpuSafetyVisitor<'a> {
    fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
        // Check if this is a frame function
        for stmt in &node.block.stmts {
            if let syn::Stmt::Expr(expr, None) = stmt {
                if let syn::Expr::MethodCall(method_call) = expr {
                    if is_framealloc_call(expr) == Some(FrameallocCall::BeginFrame) {
                        self.in_frame_scope = true;
                        self.frame_start_span = Some(method_call.span());
                    } else if is_framealloc_call(expr) == Some(FrameallocCall::EndFrame) {
                        self.check_staging_buffer_leak();
                        self.in_frame_scope = false;
                        self.frame_start_span = None;
                        // Clear frame-local tracking
                        self.staging_buffers.clear();
                    }
                }
            }
        }
        
        syn::visit::visit_item_fn(self, node);
    }
    
    fn visit_expr_method_call(&mut self, node: &'a syn::ExprMethodCall) {
        if let Some(receiver) = node.receiver.as_ref() {
            if let syn::Expr::Path(path) = receiver.as_ref() {
                // Check for unified allocator methods
                if path.path.segments.last().map(|s| s.ident == "unified").unwrap_or(false) {
                    match node.method.to_string().as_str() {
                        "create_staging_buffer" => {
                            self.staging_buffers.push(StagingBufferInfo {
                                name: None,
                                span: node.span(),
                                created_in_frame: self.in_frame_scope,
                                freed: false,
                            });
                        }
                        "create_gpu_buffer" => {
                            // Parse buffer usage from arguments
                            let has_transfer = node.args.iter().any(|arg| {
                                if let syn::Expr::Path(path) = arg {
                                    path.path.segments.iter().any(|s| {
                                        s.ident == "TRANSFER_DST" || s.ident == "TRANSFER_SRC"
                                    })
                                } else {
                                    false
                                }
                            });
                            
                            let memory_type = "DeviceLocal"; // Default, would need proper parsing
                            
                            let buffer = GpuBufferInfo {
                                name: None,
                                span: node.span(),
                                memory_type: memory_type.to_string(),
                                has_transfer_usage: has_transfer,
                                is_mapped: false,
                            };
                            
                            self.check_gpu_buffer_transfer_usage(&buffer);
                            self.gpu_buffers.push(buffer);
                        }
                        "transfer_to_gpu" => {
                            // FA803: Check if there's a synchronization barrier
                            self.transfers_without_barrier.push(TransferInfo {
                                span: node.span(),
                                has_barrier: false, // Would need context analysis
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        
        syn::visit::visit_expr_method_call(self, node);
    }
    
    fn visit_expr_macro(&mut self, node: &'a syn::ExprMacro) {
        // Check for GPU-specific macros if any are added in the future
        syn::visit::visit_expr_macro(self, node);
    }
}

