//! Bevy systems for frame lifecycle management.

use bevy_ecs::system::Res;

use crate::bevy::resource::AllocResource;

/// System that runs at the start of each frame.
///
/// Prepares the frame arena for new allocations.
pub fn begin_frame_system(alloc: Res<AllocResource>) {
    alloc.0.begin_frame();
}

/// System that runs at the end of each frame.
///
/// Resets the frame arena, invalidating all frame allocations.
pub fn end_frame_system(alloc: Res<AllocResource>) {
    alloc.0.end_frame();
}
