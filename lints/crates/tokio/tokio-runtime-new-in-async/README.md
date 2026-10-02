# tokio-runtime-new-in-async

## What it does

Checks for calls to `tokio::runtime::Runtime::new` whose nearest enclosing
function or closure is async.

## Why is this bad?

Dropping a Tokio runtime blocks until its worker threads stop. Tokio panics
when a runtime is dropped within an asynchronous execution context, so a
runtime created and dropped in async code panics when it goes out of scope.
The code is already running on a runtime that it can use.

## Known problems

The lint warns even when the runtime is moved out of the async body and
dropped in synchronous code, which does not panic.

The lint checks only `Runtime::new`. A runtime created with
`tokio::runtime::Builder::build`, or returned by a helper function, does not
trigger the lint. A call inside a synchronous closure that is defined and
called in async code does not trigger the lint either.

## Example

```rust
async fn start() -> std::io::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.spawn(work());
    Ok(())
}
```

## Use instead

```rust
async fn start() -> std::io::Result<()> {
    tokio::spawn(work());
    Ok(())
}
```
