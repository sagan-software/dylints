//! A Bevy 0.18 resource/component type for cross-version UI coverage.

use bevy_018::prelude::{Component, Reflect, Resource};

// Bevy 0.18's derive macros expect the canonical crate names. Re-export the
// facade's modules under those names so this fixture needs no extra dependencies.
pub use bevy_018::{ecs as bevy_ecs, reflect as bevy_reflect};

/// Implements both 0.18 traits whose stores are still independent.
#[derive(Component, Resource, Clone, Copy, Debug, Default, Reflect)]
pub struct OldPosition;
