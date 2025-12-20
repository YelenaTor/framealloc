//! Debug utilities for tracking allocations.
//!
//! Only compiled when the `debug` feature is enabled.

pub(crate) mod backtrace;
pub(crate) mod poison;
