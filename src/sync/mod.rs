//! Synchronization primitives.
//!
//! Provides thin wrappers over std or parking_lot mutexes.

pub(crate) mod atomics;
pub(crate) mod mutex;
