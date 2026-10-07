# tokio-zero-runtime-thread-count

## What it does

Checks `tokio::runtime::Builder::worker_threads` and
`tokio::runtime::Builder::max_blocking_threads` calls whose thread count is
statically known to be zero, including values resolved from local constants
and supported unsigned arithmetic.

## Why is this bad?

Tokio documents that both methods panic when the thread count is zero. The
mistake is visible in the source but fails only at runtime.

## Known problems

The lint resolves integer literals, local non-trait constants, and unsigned
`+`, `-`, `*`, `/`, or `%` expressions through 16 nested steps. It reports
only values it can prove are zero. Runtime locals and function calls remain
unknown, as do casts, statics, trait or external constants, unsupported
operators, and arithmetic that overflows, underflows, or divides by zero.

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
