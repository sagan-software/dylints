# bevy-component-field-contention

## What it does

Checks for a local component that two systems in the same schedule mutate through `Query<&mut T>`
when the systems write disjoint sets of its named fields.

## Why is this bad?

Bevy tracks access per component, not per field. Two systems that both take `&mut Motion` cannot
run in parallel, even when one writes only `x` and the other writes only `y`.

## Known problems

The lint compares named free functions registered through `App::add_systems` or
`SubApp::add_systems`. It resolves tuple members and schedule configuration methods such as
`.chain()`, `.run_if()`, and `.after()`.

It skips a function when its parameters contain more than one mutable local-component query access
or when the function uses a component as a whole value. Field accesses inside closures count.

## Example

```rust
# use bevy::app::{App, Update};
# use bevy::ecs::prelude::{Component, Query};
#[derive(Component)]
struct Motion {
    x: f32,
    y: f32,
}

fn move_x(mut query: Query<&mut Motion>) {
    for mut motion in &mut query {
        motion.x += 1.0;
    }
}

fn move_y(mut query: Query<&mut Motion>) {
    for mut motion in &mut query {
        motion.y += 1.0;
    }
}

fn build(app: &mut App) {
    app.add_systems(Update, (move_x, move_y));
}
```

## Use instead

```rust
# use bevy::ecs::prelude::{Component, Query};
#[derive(Component)]
struct MotionX(f32);

#[derive(Component)]
struct MotionY(f32);

fn move_x(mut query: Query<&mut MotionX>) {
    for mut motion in &mut query {
        motion.0 += 1.0;
    }
}

fn move_y(mut query: Query<&mut MotionY>) {
    for mut motion in &mut query {
        motion.0 += 1.0;
    }
}
```
