# logged_error_continue

## What it does

Checks functions, methods, and `async fn`s that return `Result<T, E>` for an
`if let Err(..)` branch or a `match` `Err(..)` arm that only logs the error
and then continues. The caught error must have the same type `E` as the
function's error.

## Why is this bad?

The function can return the error, but the branch drops it after logging.
The caller sees `Ok` and cannot retry, report, or handle the failure.

## Known problems

The branch counts as logging only when every statement is a logging call:
`eprintln!`, a `debug!`, `error!`, `event!`, `info!`, `log!`, `trace!`, or
`warn!` macro of the `log` or `tracing` crates (directly or through a local
macro that expands to one), or a function such as `warn` or `log_error` that
resolves to a `kslog`, `log`, or `tracing` crate or module. `println!` and
logging through helper functions do not count.

The lint misses:

- caught errors of a different type that `?` could convert with `From`;
- branches with any statement besides logging, including `let`;
- match arms with a guard;
- closures and `async` blocks.

## Example

```rust
fn reload() -> Result<(), std::io::Error> {
    if let Err(error) = std::fs::remove_file("cache.bin") {
        eprintln!("cache cleanup failed: {error}");
    }
    Ok(())
}
```

## Use instead

```rust
fn reload() -> Result<(), std::io::Error> {
    std::fs::remove_file("cache.bin")?;
    Ok(())
}
```
