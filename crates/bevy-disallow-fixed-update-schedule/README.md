# bevy-disallow-fixed-update-schedule

## What it does

Checks for `App::add_systems` calls whose schedule argument is `FixedUpdate`.

This lint enforces a project rule: systems run in `Update` or another schedule the project chooses.

## Why is this bad?

`FixedUpdate` runs zero, one, or several times per rendered frame. Presentation systems there, such
as animation or camera movement, change in steps that do not line up with frames and stutter.

## Known problems

The lint reports every `FixedUpdate` registration, including simulation systems that belong there.
Projects that run systems in `FixedUpdate` should allow this lint.

The lint does not see systems added through `Schedule::add_systems` or through a generic schedule
label parameter.

## Example

```rust
# use bevy::prelude::*;
# fn animate() {}
fn build(app: &mut App) {
    app.add_systems(FixedUpdate, animate);
}
```

## Use instead

```rust
# use bevy::prelude::*;
# fn animate() {}
fn build(app: &mut App) {
    app.add_systems(Update, animate);
}
```
