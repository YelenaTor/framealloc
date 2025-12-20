//! Bevy integration for framealloc.
//!
//! Provides a plugin that automatically manages frame lifecycle
//! and exposes the allocator as a Bevy resource.

mod plugin;
mod resource;
mod systems;

pub use plugin::SmartAllocPlugin;
pub use resource::AllocResource;
