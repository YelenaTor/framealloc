//! Diagnostic macros for user-facing API.
//!
//! These macros provide a clean, rustc-like diagnostic experience.

/// Emit a runtime diagnostic.
///
/// # Example
///
/// ```rust,ignore
/// fa_diagnostic!(
///     Error,
///     code = "FA001",
///     message = "frame allocation used outside an active frame",
///     note = "this allocation was requested after end_frame()",
///     help = "call alloc.begin_frame() before allocating"
/// );
/// ```
#[macro_export]
macro_rules! fa_diagnostic {
    (
        $kind:ident,
        code = $code:expr,
        message = $msg:expr
        $(, note = $note:expr)?
        $(, help = $help:expr)?
    ) => {{
        #[cfg(any(debug_assertions, feature = "diagnostics"))]
        {
            let diag = $crate::diagnostics::Diagnostic {
                kind: $crate::diagnostics::DiagnosticKind::$kind,
                code: $code,
                message: $msg,
                note: None $(.or(Some($note)))?,
                help: None $(.or(Some($help)))?,
            };
            $crate::diagnostics::emit::emit(&diag);
        }
    }};
}

/// Emit a runtime diagnostic with captured context.
///
/// # Example
///
/// ```rust,ignore
/// fa_diagnostic_ctx!(
///     Warning,
///     code = "FA003",
///     message = "frame arena exhausted"
/// );
/// ```
#[macro_export]
macro_rules! fa_diagnostic_ctx {
    (
        $kind:ident,
        code = $code:expr,
        message = $msg:expr
        $(, note = $note:expr)?
        $(, help = $help:expr)?
    ) => {{
        #[cfg(any(debug_assertions, feature = "diagnostics"))]
        {
            let diag = $crate::diagnostics::Diagnostic {
                kind: $crate::diagnostics::DiagnosticKind::$kind,
                code: $code,
                message: $msg,
                note: None $(.or(Some($note)))?,
                help: None $(.or(Some($help)))?,
            };
            let ctx = $crate::diagnostics::context::DiagContext::capture();
            $crate::diagnostics::emit::emit_with_context(&diag, &ctx.format());
        }
    }};
}

/// Emit a predefined diagnostic by code.
///
/// # Example
///
/// ```rust,ignore
/// fa_emit!(FA001);
/// fa_emit!(FA101);
/// ```
#[macro_export]
macro_rules! fa_emit {
    ($code:ident) => {{
        #[cfg(any(debug_assertions, feature = "diagnostics"))]
        {
            $crate::diagnostics::emit::emit(&$crate::diagnostics::$code);
        }
    }};
}

/// Emit a predefined diagnostic with context.
#[macro_export]
macro_rules! fa_emit_ctx {
    ($code:ident) => {{
        #[cfg(any(debug_assertions, feature = "diagnostics"))]
        {
            let ctx = $crate::diagnostics::context::DiagContext::capture();
            $crate::diagnostics::emit::emit_with_context(
                &$crate::diagnostics::$code,
                &ctx.format(),
            );
        }
    }};
}

/// Compile-time diagnostic error.
///
/// This produces a hard compiler error with a formatted message.
///
/// # Example
///
/// ```rust,ignore
/// fa_compile_error!(
///     code = "FA101",
///     message = "Bevy support enabled but plugin not registered",
///     help = "add .add_plugins(SmartAllocPlugin) to your App"
/// );
/// ```
#[macro_export]
macro_rules! fa_compile_error {
    (
        code = $code:expr,
        message = $msg:expr
        $(, help = $help:expr)?
    ) => {
        compile_error!(concat!(
            "[framealloc][", $code, "] ", $msg
            $(, "\n  help: ", $help)?
        ));
    };
}

/// Compile-time diagnostic warning (via deprecated).
///
/// This produces a compiler warning using the deprecation mechanism.
#[macro_export]
macro_rules! fa_compile_warning {
    (
        code = $code:expr,
        message = $msg:expr
    ) => {
        #[deprecated(note = concat!("[framealloc][", $code, "] ", $msg))]
        const _FRAMEALLOC_WARNING: () = ();
        let _ = _FRAMEALLOC_WARNING;
    };
}

/// Assert a condition or emit a diagnostic.
///
/// # Example
///
/// ```rust,ignore
/// fa_assert!(frame_active, FA001);
/// ```
#[macro_export]
macro_rules! fa_assert {
    ($cond:expr, $code:ident) => {{
        #[cfg(any(debug_assertions, feature = "diagnostics"))]
        {
            if !$cond {
                $crate::fa_emit!($code);
            }
        }
    }};
    ($cond:expr, $code:ident, ctx) => {{
        #[cfg(any(debug_assertions, feature = "diagnostics"))]
        {
            if !$cond {
                $crate::fa_emit_ctx!($code);
            }
        }
    }};
}

/// Debug-only diagnostic (completely removed in release).
#[macro_export]
macro_rules! fa_debug {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            $crate::fa_diagnostic!($($arg)*);
        }
    }};
}

// Re-export macros at crate root for convenience
pub use crate::{fa_assert, fa_compile_error, fa_compile_warning, fa_debug, fa_diagnostic, fa_diagnostic_ctx, fa_emit, fa_emit_ctx};
