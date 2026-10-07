# tokio-spawn-blocking-async-closure

## What it does

Checks calls to `tokio::task::spawn_blocking` when its function argument returns a type that
implements `Future`.

## Why is this bad?

Calling such a closure creates a future but does not poll it.
`spawn_blocking` runs the closure on a blocking thread and returns the
unpolled future as the task output. The async work never runs unless the
caller awaits that returned future.

## Known problems

The lint checks the resolved direct call and its inferred result type. It does not track whether the
caller later awaits the `JoinHandle` and polls its future result.

## Example

```rust
# async fn work() {}
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
# async fn work() {}
fn start() {
    let _task = tokio::spawn(async {
        work().await;
    });
}
```
