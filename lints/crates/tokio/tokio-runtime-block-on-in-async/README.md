# tokio-runtime-block-on-in-async

## What it does

Checks for calls to `tokio::runtime::Runtime::block_on` whose nearest
enclosing function or closure is async.

## Why is this bad?

Tokio documents that `Runtime::block_on` panics when called within an
asynchronous execution context. Async code can await the future directly.

## Known problems

The lint treats every async body as running inside a Tokio runtime. It warns
on a future that is polled by another executor.

The lint stops at the nearest closure. A call inside a synchronous closure
that is defined and called in async code does not trigger the lint. Calls
inside a synchronous helper function do not trigger the lint either.

## Example

```rust
async fn load(runtime: &tokio::runtime::Runtime) -> u32 {
    runtime.block_on(fetch())
}
```

## Use instead

```rust
async fn load() -> u32 {
    fetch().await
}
```
