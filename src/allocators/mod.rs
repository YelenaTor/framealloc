//! Allocation backends.
//!
//! This module contains the core allocator implementations.
//! **These are the only modules that should contain `unsafe` code.**

pub(crate) mod deferred;
pub(crate) mod frame;
pub(crate) mod handles;
pub(crate) mod heap;
pub(crate) mod slab;
pub(crate) mod streaming;
