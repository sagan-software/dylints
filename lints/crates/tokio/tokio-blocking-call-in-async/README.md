# tokio-blocking-call-in-async

## What it does

Checks for calls to Tokio's `blocking_lock`, `blocking_lock_owned`,
`blocking_read`, `blocking_read_owned`, `blocking_write`,
`blocking_write_owned`, `blocking_send`, and `blocking_recv` methods whose
nearest enclosing function or closure is async.

## Why is this bad?

Tokio documents that these methods panic when called within an asynchronous
execution context. They exist for synchronous code that shares a Tokio
primitive with async code.

## Known problems

The lint treats every async body as running inside a Tokio runtime. It warns
on a future that is polled by another executor, where the call does not panic.

The lint stops at the nearest closure. A call inside a synchronous closure
that is defined and called in async code does not trigger the lint, even
though the call still runs in the async context. Calls inside a synchronous
helper function do not trigger the lint either.

## Example

```rust
async fn read_value(value: &tokio::sync::Mutex<u8>) -> u8 {
    *value.blocking_lock()
}
```

## Use instead

Await the asynchronous method. If the whole operation is synchronous, move it
into a `tokio::task::spawn_blocking` closure instead.

```rust
async fn read_value(value: &tokio::sync::Mutex<u8>) -> u8 {
    *value.lock().await
}
```
