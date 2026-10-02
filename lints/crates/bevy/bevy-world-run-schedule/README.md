# bevy-world-run-schedule

## What it does

Checks for calls to `World::run_schedule`.

## Why is this bad?

`World::run_schedule` panics when no schedule with that label exists. A label that was never added
then stops the whole app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the schedule
exists.

## Example

```rust
fn run_gameplay(world: &mut World) {
    world.run_schedule(Gameplay);
}
```

## Use instead

```rust
fn run_gameplay(world: &mut World) -> Result<(), BevyError> {
    world.try_run_schedule(Gameplay)?;
    Ok(())
}
```
