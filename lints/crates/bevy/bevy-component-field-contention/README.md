# bevy-component-field-contention

## What it does

Checks for a local component that two systems in the same schedule mutate through `Query<&mut T>`
when the systems write disjoint sets of its named fields.

## Why is this bad?

Bevy tracks access per component, not per field. Two systems that both take `&mut Motion` cannot
run in parallel, even when one writes only `x` and the other writes only `y`.

## Known problems

The lint only sees free functions passed to `App::add_systems` as a plain path or in a tuple of
paths. It skips systems with ordering or run conditions, such as `(move_x, move_y).chain()`.

It skips a function when its queries mutate more than one local component, or when the function
uses the component as a whole value. Field accesses inside closures are not seen.

## Example

```rust
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
