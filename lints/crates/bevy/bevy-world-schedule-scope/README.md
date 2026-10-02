# bevy-world-schedule-scope

## What it does

Checks for calls to `World::schedule_scope`.

## Why is this bad?

`World::schedule_scope` panics when no schedule with that label exists. A label that was never
added then stops the whole app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the schedule
exists.

## Example

```rust
fn run_gameplay(world: &mut World) {
    world.schedule_scope(Gameplay, |world, schedule| schedule.run(world));
}
```

## Use instead

```rust
fn run_gameplay(world: &mut World) -> Result<(), BevyError> {
    world.try_schedule_scope(Gameplay, |world, schedule| schedule.run(world))?;
    Ok(())
}
```
