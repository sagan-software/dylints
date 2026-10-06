# bevy-mouse-displacement-times-delta

## What it does

Warns when resolved `MouseMotion::delta` or `AccumulatedMouseMotion::delta` is multiplied by resolved Bevy frame delta time in seconds.

## Why is this bad?

Mouse delta already measures movement during the frame. Multiplying it by elapsed seconds makes camera sensitivity depend on frame duration.

## Known problems

The lint checks local multiplication expressions using Bevy's mouse-motion fields and non-fixed `Time` delta methods: `delta_secs`, `delta_secs_f64`, or `delta().as_secs_f32`/`as_secs_f64`. It excludes resolved `Time<Fixed>` deltas because they represent fixed-step duration. It leaves rate conversion and direct rate-to-displacement reconstruction clean. Keyboard velocity multiplied by delta time is valid and does not trigger this lint.

The lint follows value-producing arithmetic, casts, field access and indexing, block tails, and branch results. It skips conditions, match selectors and guards, and discarded block statements. It does not trace values through locals, function calls, or custom wrappers.

## Example

```rust
# use bevy::ecs::{message::MessageReader, system::Res};
# use bevy::input::mouse::MouseMotion;
# use bevy::time::Time;

fn rotate_camera(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        let _rotation = event.delta.x * time.delta_secs();
    }
}
```

## Use instead

```rust
# use bevy::ecs::{
#     message::MessageReader,
#     resource::Resource,
#     system::Res,
# };
# use bevy::input::mouse::MouseMotion;
# use bevy::time::Time;

#[derive(Resource)]
struct MouseSensitivity(f32);

fn rotate_camera(mut motion: MessageReader<MouseMotion>, sensitivity: Res<MouseSensitivity>) {
    for event in motion.read() {
        let _rotation = event.delta.x * sensitivity.0;
    }
}
```
