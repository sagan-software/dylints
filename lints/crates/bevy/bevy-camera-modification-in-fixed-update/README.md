# bevy-camera-modification-in-fixed-update

## What it does

Checks for systems added to `FixedUpdate` through `App::add_systems` when the system has a query
with mutable data and a `With<Camera>` filter.

## Why is this bad?

`FixedUpdate` runs zero, one, or several times per rendered frame. A camera moved there changes in
fixed steps that do not line up with frames, so the view stutters.

## Known problems

The lint only checks systems passed as a plain function path or in a tuple of paths. It skips
systems with ordering or run conditions, such as `move_camera.after(input)`. It only recognizes the
`With<Camera>` filter, so the lint does not report queries filtered on `Camera2d` or `Camera3d`.

## Example

```rust
fn move_camera(mut cameras: Query<&mut Transform, With<Camera>>) {
    for mut transform in &mut cameras {
        transform.translation.x += 1.0;
    }
}

fn build(app: &mut App) {
    app.add_systems(FixedUpdate, move_camera);
}
```

## Use instead

```rust
fn build(app: &mut App) {
    app.add_systems(Update, move_camera);
}
```
