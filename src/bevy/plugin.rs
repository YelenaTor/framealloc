//! Bevy plugin for framealloc.

use bevy_app::{App, First, Last, Plugin};

use crate::api::config::AllocConfig;
use crate::api::alloc::SmartAlloc;
use crate::bevy::resource::AllocResource;
use crate::bevy::systems::{begin_frame_system, end_frame_system};

/// Bevy plugin that integrates framealloc into your game.
///
/// # Example
///
/// ```rust,ignore
/// use bevy::prelude::*;
/// use framealloc::bevy::SmartAllocPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(SmartAllocPlugin::default())
///     .run();
/// ```
pub struct SmartAllocPlugin {
    config: AllocConfig,
}

impl SmartAllocPlugin {
    /// Create a new plugin with the given configuration.
    pub fn new(config: AllocConfig) -> Self {
        Self { config }
    }

    /// Create a plugin with high-performance settings.
    pub fn high_performance() -> Self {
        Self {
            config: AllocConfig::high_performance(),
        }
    }

    /// Create a plugin with minimal settings (for testing).
    pub fn minimal() -> Self {
        Self {
            config: AllocConfig::minimal(),
        }
    }
}

impl Default for SmartAllocPlugin {
    fn default() -> Self {
        Self {
            config: AllocConfig::default(),
        }
    }
}

impl Plugin for SmartAllocPlugin {
    fn build(&self, app: &mut App) {
        let alloc = SmartAlloc::new(self.config.clone());

        app.insert_resource(AllocResource(alloc))
            .add_systems(First, begin_frame_system)
            .add_systems(Last, end_frame_system);
    }
}
