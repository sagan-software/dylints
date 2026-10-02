# tokio-spawn-blocking-async-closure

## What it does

Checks for calls to `tokio::task::spawn_blocking` whose argument is an async
closure or a closure whose body is an async block.

## Why is this bad?

Calling such a closure creates a future but does not poll it.
`spawn_blocking` runs the closure on a blocking thread and returns the
unpolled future as the task output. The async work never runs unless the
caller awaits that returned future.

## Known problems

The lint checks only a closure written directly as the argument. A closure
stored in a variable first, or a closure that calls a function returning a
future, does not trigger the lint.

## Example

```rust
fn start() {
    let _task = tokio::task::spawn_blocking(|| async {
        work().await;
    });
}
```

## Use instead

Spawn async work with `tokio::spawn`. If the work is blocking, keep the
`spawn_blocking` closure synchronous.

```rust
fn start() {
    let _task = tokio::spawn(async {
        work().await;
    });
}
```
