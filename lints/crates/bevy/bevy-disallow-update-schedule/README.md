# bevy-disallow-update-schedule

## What it does

Checks for `App::add_systems` calls whose schedule argument is `Update`.

This lint enforces a project rule: simulation systems run in `FixedUpdate` or another schedule the
project chooses.

## Why is this bad?

`Update` runs once per rendered frame with a variable time step. Simulation in `Update` produces
results that depend on frame rate, which breaks deterministic replays and networked lockstep.

## Known problems

The lint reports every `Update` registration, including presentation systems that belong there.
Projects that run systems in `Update` should allow this lint.

The lint does not see systems added through `Schedule::add_systems` or through a generic schedule
label parameter.

## Example

```rust
# use bevy::prelude::*;
# fn tick() {}
fn build(app: &mut App) {
    app.add_systems(Update, tick);
}
```

## Use instead

```rust
# use bevy::prelude::*;
# fn tick() {}
fn build(app: &mut App) {
    app.add_systems(FixedUpdate, tick);
}
```
