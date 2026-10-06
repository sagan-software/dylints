#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    unknown_lints,
    unused_results,
    unused_crate_dependencies,
    reason = "UI examples retain unused definitions to exercise type resolution"
)]

//! Exercise Bevy text span and layout recognition with compile-time fixtures.

use bevy::ecs::{
    bundle::Bundle, component::Component, entity::Entity, system::Commands, world::World,
};
use bevy::sprite::Text2d;
use bevy::text::{TextLayout as Layout, TextSpan as Span};
use bevy::ui::widget::Text;

/// Alias used to verify the resolved Bevy text-span type.
type TextSpan = Span;
/// Alias used to verify the resolved Bevy text-layout type.
type TextLayout = Layout;

/// Local components that share Bevy type names.
mod lookalikes {
    use super::Component;

    /// A local component named like Bevy's `TextSpan`.
    #[derive(Component)]
    pub(super) struct TextSpan;

    /// A local component named like Bevy's `TextLayout`.
    #[derive(Component)]
    pub(super) struct TextLayout;
}

#[derive(Bundle)]
/// A custom bundle whose fields remain opaque to this lint.
struct OpaqueTextBundle {
    /// The Bevy text span in the bundle.
    span: Span,
    /// The Bevy text layout in the bundle.
    layout: Layout,
}

/// Exercise root text layouts and direct span-layout bundles.
fn text_bundles(world: &mut World) {
    world.spawn((Span::new("span"), Layout::default()));
    world.spawn((Span::new("nested"), (Layout::default(),)));
    world.spawn((TextSpan::new("aliased span"), TextLayout::default()));

    // Root text layouts are valid.
    world.spawn((Text::new("UI root"), Layout::default()));
    world.spawn((Text2d::new("2D root"), Layout::default()));

    // A span without an entity-local layout remains valid.
    world.spawn(Span::new("child"));
    let later_configured_span = world.spawn(Span::new("later layout")).id();
    world
        .entity_mut(later_configured_span)
        .insert(Layout::default());
    world.spawn((lookalikes::TextSpan, lookalikes::TextLayout));
    world.spawn(OpaqueTextBundle {
        span: Span::new("opaque"),
        layout: Layout::default(),
    });
    world
        .spawn_empty()
        .insert((Span::new("inserted span"), Layout::default()));
    world
        .spawn_empty()
        .insert_if_new((Span::new("inserted if new"), Layout::default()));
    world
        .spawn_empty()
        .with_related_entities::<bevy::ecs::hierarchy::ChildOf>(|related| {
            related.spawn((Span::new("related span"), Layout::default()));
        });
}

/// Exercise command-buffer bundle methods.
fn commands(mut commands: Commands, entity: Entity) {
    commands.spawn((Span::new("span"), Layout::default()));
    commands
        .entity(entity)
        .insert((Span::new("inserted"), Layout::default()));
    commands
        .entity(entity)
        .insert_if_new((Span::new("inserted if new"), Layout::default()));
    commands
        .spawn_empty()
        .with_related_entities::<bevy::ecs::hierarchy::ChildOf>(|related| {
            related.spawn((Span::new("related span"), Layout::default()));
        });
}

/// Type-check the UI fixture without loading text assets.
fn main() {}
