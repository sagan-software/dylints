#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    unknown_lints,
    unused_results,
    unused_crate_dependencies,
    reason = "UI examples retain unused definitions to exercise type resolution"
)]

//! Exercise Bevy MSAA bundle recognition with compile-time fixtures.

use bevy_camera::{Camera, Camera2d, Camera3d};
use bevy_core_pipeline::{
    oit::OrderIndependentTransparencySettings, prepass::DeferredPrepass as RealDeferredPrepass,
};
use bevy_ecs::{
    bundle::Bundle, component::Component, entity::Entity, system::Commands, world::World,
};
use bevy_pbr::ScreenSpaceAmbientOcclusion;
use bevy_render::view::Msaa as RenderMsaa;

/// Alias used to verify resolved MSAA enum paths.
type Samples = RenderMsaa;
/// Alias used to verify resolved deferred-prepass paths.
type Deferred = RealDeferredPrepass;
/// Alias used to verify resolved SSAO component paths.
type AmbientOcclusion = ScreenSpaceAmbientOcclusion;
/// Alias used to verify resolved OIT settings paths.
type Transparency = OrderIndependentTransparencySettings;

/// A compile-time MSAA value that this lint deliberately does not resolve.
const PRESET_MSAA: RenderMsaa = RenderMsaa::Sample4;

/// A local component with the Bevy MSAA type name.
#[derive(Component)]
struct Msaa;

/// A local component with the Bevy deferred-prepass type name.
#[derive(Component)]
struct DeferredPrepass;

#[derive(Bundle)]
/// A custom bundle whose fields remain opaque to this lint.
struct OpaqueCameraBundle {
    /// The camera included in this bundle.
    camera: Camera3d,
    /// The sample count included in this bundle.
    msaa: RenderMsaa,
    /// The deferred prepass included in this bundle.
    deferred: RealDeferredPrepass,
}

/// Exercise direct Bevy bundles and their conservative boundaries.
fn camera_bundles(world: &mut World) {
    incompatible_feature_bundles(world);
    valid_msaa_values(world);
    separate_and_lookalike_components(world);
    camera_query_boundaries(world);
    world_entity_mut_bundles(world);
}

/// Exercise the three runtime incompatibilities, including OIT's unfiltered check.
fn incompatible_feature_bundles(world: &mut World) {
    world.spawn((
        Camera3d::default(),
        RenderMsaa::Sample2,
        RealDeferredPrepass,
    ));
    world.spawn((
        Camera3d::default(),
        Samples::Sample4,
        AmbientOcclusion::default(),
    ));
    world.spawn((
        Camera3d::default(),
        RenderMsaa::Sample8,
        Transparency::default(),
    ));
    // Bevy 0.19.0's OIT check has no camera filter.
    world.spawn((RenderMsaa::Sample2, Transparency::default()));
}

/// Exercise disabled, inferred, computed, and locally bound sample counts.
fn valid_msaa_values(world: &mut World) {
    world.spawn((Camera3d::default(), RenderMsaa::Off, Deferred::default()));
    world.spawn((
        Camera3d::default(),
        RenderMsaa::default(),
        RealDeferredPrepass,
    ));
    world.spawn((
        Camera3d::default(),
        RenderMsaa::from_samples(4),
        RealDeferredPrepass,
    ));
    world.spawn((Camera3d::default(), PRESET_MSAA, RealDeferredPrepass));
    let local_samples = RenderMsaa::Sample4;
    world.spawn((Camera3d::default(), local_samples, RealDeferredPrepass));
}

/// Exercise separate-entity, lookalike, and later-insertion boundaries.
fn separate_and_lookalike_components(world: &mut World) {
    // A marker and MSAA on separate entities do not form one bundle.
    world.spawn((Camera3d::default(), RenderMsaa::Sample4));
    world.spawn(DeferredPrepass);

    // Exact Bevy type resolution keeps same-named local components clean.
    world.spawn((Camera3d::default(), Msaa, DeferredPrepass));

    let later_configured_camera = world.spawn(Camera3d::default()).id();
    world
        .entity_mut(later_configured_camera)
        .insert(RenderMsaa::Sample4);
    world
        .entity_mut(later_configured_camera)
        .insert(RealDeferredPrepass);
}

/// Exercise the camera filters used by deferred rendering and SSAO.
fn camera_query_boundaries(world: &mut World) {
    world.spawn((Camera2d, RenderMsaa::Sample2, RealDeferredPrepass));
    // The SSAO extraction query requires Camera3d, so this combination is clean.
    world.spawn((Camera2d, RenderMsaa::Sample4, AmbientOcclusion::default()));
    world.spawn((Camera::default(), RenderMsaa::Sample2, RealDeferredPrepass));
}

/// Exercise custom bundles and direct entity mutation methods.
fn world_entity_mut_bundles(world: &mut World) {
    world.spawn(OpaqueCameraBundle {
        camera: Camera3d::default(),
        msaa: RenderMsaa::Sample4,
        deferred: RealDeferredPrepass,
    });
    world
        .spawn(Camera3d::default())
        .insert((RenderMsaa::Sample4, RealDeferredPrepass));
    world.spawn_empty().insert_if_new((
        Camera3d::default(),
        RenderMsaa::Sample4,
        RealDeferredPrepass,
    ));
    world
        .spawn_empty()
        .with_related_entities::<bevy_ecs::hierarchy::ChildOf>(|related| {
            related.spawn((
                Camera3d::default(),
                RenderMsaa::Sample4,
                RealDeferredPrepass,
            ));
        });
}

/// Exercise command-buffer bundle methods.
fn commands(mut commands: Commands, entity: Entity) {
    commands.spawn((
        Camera3d::default(),
        RenderMsaa::Sample4,
        RealDeferredPrepass,
    ));
    commands.entity(entity).insert((
        Camera3d::default(),
        RenderMsaa::Sample4,
        RealDeferredPrepass,
    ));
    commands.entity(entity).insert_if_new((
        Camera3d::default(),
        RenderMsaa::Sample4,
        RealDeferredPrepass,
    ));
    commands
        .spawn_empty()
        .with_related_entities::<bevy_ecs::hierarchy::ChildOf>(|related| {
            related.spawn((
                Camera3d::default(),
                RenderMsaa::Sample4,
                RealDeferredPrepass,
            ));
        });
}

/// Type-check the UI fixture without running a renderer.
fn main() {}
