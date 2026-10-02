# bevy-large-component

## What it does

Checks for local components with at least eight named fields and a size over 64 bytes.

## Why is this bad?

Bevy tracks access and changes per component. A system that needs one field of a large component
borrows all of it, so it conflicts with every system that writes any other field. Changing one
field also marks the whole component as changed.

## Known problems

The lint uses fixed thresholds. It still reports some cohesive components that exceed them.

The lint skips resources, tuple structs, enums, and generic components.

## Example

```rust
#[derive(Component)]
struct Agent {
    position: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
    health: f32,
    stamina: f32,
    hunger: f32,
    target: Option<Entity>,
    path_index: usize,
    team: u32,
}
```

## Use instead

```rust
#[derive(Component)]
struct Kinematics {
    position: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
}

#[derive(Component)]
struct Vitals {
    health: f32,
    stamina: f32,
    hunger: f32,
}

#[derive(Component)]
struct Navigation {
    target: Option<Entity>,
    path_index: usize,
}

#[derive(Component)]
struct Team(u32);
```
