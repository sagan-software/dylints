# bevy-world-schedule-scope

## What it does

Checks for calls to `World::schedule_scope`.

## Why is this bad?

`World::schedule_scope` panics when no schedule with that label exists. When code never adds the label, `World::schedule_scope` stops the whole app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the schedule
exists.

## Example

```rust
# use bevy::prelude::*;
# use bevy::ecs::{error::BevyError, schedule::ScheduleLabel};
# #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)] struct Gameplay;
fn run_gameplay(world: &mut World) {
    world.schedule_scope(Gameplay, |world, schedule| schedule.run(world));
}
```

## Use instead

```rust
# use bevy::prelude::*;
# use bevy::ecs::{error::BevyError, schedule::ScheduleLabel};
# #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)] struct Gameplay;
fn run_gameplay(world: &mut World) -> Result<(), BevyError> {
    world.try_schedule_scope(Gameplay, |world, schedule| schedule.run(world))?;
    Ok(())
}
```
