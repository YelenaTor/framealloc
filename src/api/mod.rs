//! Public API for framealloc.
//!
//! This module contains all user-facing types and functions.
//! Most users should only interact with types from this module.

pub mod alloc;
pub mod allocator_impl;
pub mod checkpoint;
pub mod config;
pub mod frame_collections;
pub mod groups;
pub mod phases;
pub mod promotion;
pub mod retention;
pub mod scope;
pub mod scratch;
pub mod stats;
pub mod tag;
pub mod tagged;
pub mod wrappers;
