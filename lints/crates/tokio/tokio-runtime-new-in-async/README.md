# tokio-runtime-new-in-async

## What it does

Checks for calls to `tokio::runtime::Runtime::new` whose nearest enclosing
function or closure is async.

## Why is this bad?

Tokio panics when code drops a runtime within an asynchronous execution context.
A runtime created and dropped in async code therefore panics when it goes out of
scope. Dropping a runtime also blocks until its worker threads stop. The code
already runs on a runtime that it can use.

## Known problems

The lint warns even when code moves the runtime out of the async body and drops
it in synchronous code, where it does not panic.

The lint checks only `Runtime::new`. A runtime created with
`tokio::runtime::Builder::build`, or returned by another function, does not
trigger the lint. A synchronous closure that async code defines and calls does
not trigger the lint either.

## Example

```rust
# async fn work() {}
async fn start() -> std::io::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.spawn(work());
    Ok(())
}
```

## Use instead

```rust
# async fn work() {}
async fn start() -> std::io::Result<()> {
    tokio::spawn(work());
    Ok(())
}
```
