//! Public API for framealloc.
//!
//! This module contains all user-facing types and functions.
//! Most users should only interact with types from this module.

pub mod alloc;
pub mod allocator_impl;
pub mod config;
pub mod groups;
pub mod scope;
pub mod stats;
pub mod tag;
pub mod wrappers;
