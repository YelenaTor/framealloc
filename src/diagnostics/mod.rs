//! Diagnostics and UI integration hooks.
//!
//! This module provides:
//! - **Runtime diagnostics**: Allocator-aware error messages with codes
//! - **UI integration**: Hooks for imgui, egui, or custom overlays
//! - **Profiler integration**: Tracy and custom profiler support
//! - **Strict mode**: Optional panic-on-error for CI
//!
//! ## Diagnostic Codes
//!
//! | Code  | Meaning                        |
//! |-------|--------------------------------|
//! | FA0xx | Frame allocation issues        |
//! | FA1xx | Bevy integration issues        |
//! | FA2xx | Threading issues               |
//! | FA3xx | Budget/limit issues            |
//! | FA4xx | Handle/streaming issues        |
//! | FA9xx | Internal errors                |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use framealloc::{fa_diagnostic, fa_emit};
//!
//! // Emit a custom diagnostic
//! fa_diagnostic!(
//!     Error,
//!     code = "FA001",
//!     message = "frame allocation outside frame",
//!     help = "call begin_frame() first"
//! );
//!
//! // Emit a predefined diagnostic
//! fa_emit!(FA001);
//! ```

// Core diagnostic types
pub mod kind;
pub mod emit;
pub mod context;
pub mod strict;
pub mod macros;

// UI integration
mod hooks;
mod snapshot;
mod tracy;

// Re-export core types
pub use kind::{Diagnostic, DiagnosticKind};
pub use emit::{emit, emit_with_context, suppress_diagnostics, set_verbose, DiagnosticSink, CollectingSink};
pub use context::{DiagContext, set_bevy_context, is_bevy_context, increment_frame, frame_number};
pub use strict::{StrictMode, set_strict_mode, strict_mode, StrictModeGuard, init_from_env};

// Re-export predefined diagnostics
pub use kind::{FA001, FA002, FA003, FA101, FA102, FA201, FA202, FA301, FA302, FA401, FA402, FA901};

// UI hooks
pub use hooks::{DiagnosticsHooks, DiagnosticsEvent, SharedDiagnostics, MemoryGraphData};
pub use snapshot::{AllocatorSnapshot, FrameSnapshot, PoolSnapshot, TagSnapshot, GlobalSnapshot, StreamingSnapshot, SnapshotHistory};
pub use tracy::{ProfilerHooks, ProfilerZone, MemoryEvent};
