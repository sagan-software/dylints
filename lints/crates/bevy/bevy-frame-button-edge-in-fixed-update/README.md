# bevy-frame-button-edge-in-fixed-update

## What it does

Warns when a local system directly registered with Bevy's `FixedUpdate` reads `just_pressed` or `just_released` from `Res<ButtonInput<T>>` or `ResMut<ButtonInput<T>>`.

## Why is this bad?

With Bevy's standard `InputPlugin`, `ButtonInput` updates in `PreUpdate`, once per rendered frame. The fixed loop can run zero or several times after that update. A fixed system can miss an edge when no tick runs or process the same edge more than once when several ticks run.

## Known problems

The lint checks direct local function registrations and cannot prove that `InputPlugin` is installed. It recognizes standalone resource parameters and resource parameters inside tuples. It skips `Option`, `ParamSet`, custom system parameters, and other parameter wrappers. It skips nested closure bodies because it cannot prove when they run.

It ignores system run conditions. Bevy's resolved `run_once` condition limits reads to at most once, but it cannot prevent edge loss when a rendered frame has no fixed tick: the standard input systems clear and rebuild `ButtonInput` in the next `PreUpdate` after its one-frame edge. Manually updated `ButtonInput` resources can be valid when their edges are maintained for each fixed tick.

Held-state reads such as `pressed` are not frame edges and do not trigger it. Custom buffered input is outside its analysis. Clearing an edge inside `FixedUpdate` can prevent repeated reads after a tick, but it cannot preserve an edge across a frame with no fixed tick. This semantic warning complements `bevy-disallow-fixed-update-schedule`, which enforces a project-wide policy.

## Example

```rust
# use bevy_app::{App, FixedUpdate};
# use bevy_ecs::system::Res;
# use bevy_input::{ButtonInput, keyboard::KeyCode};
# let mut app = App::new();

fn jump(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {
        // Apply the jump.
    }
}

app.add_systems(FixedUpdate, jump);
```

## Use instead

```rust
# use bevy_app::{
#     App, FixedUpdate, RunFixedMainLoop, RunFixedMainLoopSystems,
# };
# use bevy_ecs::{
#     prelude::IntoScheduleConfigs,
#     resource::Resource,
#     system::{Res, ResMut},
# };
# use bevy_input::{ButtonInput, keyboard::KeyCode};
# let mut app = App::new();

#[derive(Resource, Default)]
struct FixedInput {
    jump: bool,
}

fn capture_jump(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<FixedInput>) {
    input.jump |= keys.just_pressed(KeyCode::Space);
}

fn apply_jump(mut input: ResMut<FixedInput>) {
    if core::mem::take(&mut input.jump) {
        // Apply the jump once.
    }
}

app.add_systems(
    RunFixedMainLoop,
    capture_jump.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
);
app.add_systems(FixedUpdate, apply_jump);
```

Register the capture system in `RunFixedMainLoopSystems::BeforeFixedMainLoop`, which runs after `PreUpdate`. If capture runs in `PreUpdate` instead, order it after `bevy_input::InputSystems`. The boolean buffer preserves one pending jump across zero-tick frames and consumes it once, but it coalesces multiple presses while one jump is pending.
