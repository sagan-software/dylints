# bevy-incompatible-msaa

## What it does

Warns when a direct tuple bundle passed to a recognized Bevy ECS `spawn`, `insert`, `insert_if_new`, or related-spawner `spawn` method combines an explicit enabled `Msaa::Sample2`, `Msaa::Sample4`, or `Msaa::Sample8` with a render feature that conflicts with multisampling.

Deferred rendering is checked only when the bundle also contains Bevy `Camera`, `Camera2d`, or `Camera3d`. SSAO is checked only when it contains `Camera3d` and `ScreenSpaceAmbientOcclusion`. In Bevy 0.19.0, the OIT MSAA check selects entities with `OrderIndependentTransparencySettings` without a camera filter, so a bundle with OIT settings and enabled MSAA is reported even when it omits a camera.

## Why is this bad?

When the Bevy 0.19.0 Core3d pipeline checks a matching camera with `DeferredPrepass`, it turns that camera's MSAA off. When `ScreenSpaceAmbientOcclusionPlugin` extracts a matching `Camera3d` with enabled MSAA, it logs an error and returns from the extraction system. `ScreenSpaceAmbientOcclusion` requires both depth and normal prepasses. When `OrderIndependentTransparencyPlugin` checks an entity with OIT settings and more than one MSAA sample, it panics.

The 0.19.0 SSAO extraction system's early return also stops extraction for later matching cameras in that pass. Bevy 0.19.1 changed that path to continue with the next camera.

## Known problems

The runtime consequences require the corresponding pipeline or plugin system to run. The lint does not inspect the app's plugin setup, runtime state, or later changes to an entity.

The lint recognizes explicit `Msaa::Sample2`, `Msaa::Sample4`, and `Msaa::Sample8` paths in direct tuple bundles. It does not infer `Msaa::default()`, `Msaa::from_samples`, local variables, custom bundles, or component combinations assembled across separate calls. It reports the components in one bundle expression and does not prove the entity's final configuration.

The lint does not evaluate whether `insert_if_new` applies a bundle to the current entity.

The runtime details here are checked against Bevy 0.19.0: the [Core3d MSAA check](https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_core_pipeline/src/core_3d/mod.rs), [SSAO extraction](https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_pbr/src/ssao/mod.rs), and [OIT MSAA check](https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_core_pipeline/src/oit/mod.rs). Later Bevy versions may change their checks or consequences.

## Example

This OIT bundle is reported in Bevy 0.19.0 even though it has no camera, because the plugin's MSAA check filters only on its settings component.

```rust
use bevy_core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy_ecs::world::World;
use bevy_render::view::Msaa;

fn main() {
    let mut world = World::new();
    world.spawn((Msaa::Sample4, OrderIndependentTransparencySettings::default()));
}
```

## Use instead

Set `Msaa::Off` on the same entity as each incompatible render component. Deferred rendering and SSAO require a matching camera; Bevy's standard OIT setup puts its settings on the camera.

```rust
use bevy_camera::Camera3d;
use bevy_core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy_ecs::world::World;
use bevy_render::view::Msaa;

fn main() {
    let mut world = World::new();
    world.spawn((
        Camera3d::default(),
        Msaa::Off,
        OrderIndependentTransparencySettings::default(),
    ));
}
```
