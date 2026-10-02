# tokio-zero-runtime-thread-count

## What it does

Checks for the integer literal `0` passed to
`tokio::runtime::Builder::worker_threads` or
`tokio::runtime::Builder::max_blocking_threads`.

## Why is this bad?

Tokio documents that both methods panic when the thread count is zero. The
mistake is visible in the source but fails only at runtime.

## Known problems

The lint checks only the literal `0`. A constant, variable, or arithmetic
expression whose value is zero does not trigger the lint.

## Example

```rust
fn build_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(0)
        .build()
}
```

## Use instead

Use a positive thread count, or remove the call to keep Tokio's default.

```rust
fn build_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
}
```
