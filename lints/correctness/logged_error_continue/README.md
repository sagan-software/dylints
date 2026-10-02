# logged_error_continue

## What it does

Checks functions that return `Result<T, E>` for an `if let Err(..)` branch or
a `match` `Err(..)` arm that only logs the error and then continues. The
caught error must have the same type `E` as the function's error.

## Why is this bad?

The function can return the error, but the branch drops it after logging.
The caller sees `Ok` and cannot retry, report, or handle the failure.

## Known problems

The branch counts as logging only when every statement is a call to a macro
named `debug!`, `eprintln!`, `error!`, `info!`, `trace!`, or `warn!`, or a
function such as `log::warn(..)` or `tracing::error(..)` from a `kslog`,
`log`, or `tracing` path. A user-defined macro with one of these names also
counts. `println!` and logging through helper functions do not count.

The lint misses:

- caught errors of a different type that `?` could convert with `From`;
- branches with any statement besides logging, including `let`;
- match arms with a guard;
- closures, `async fn` bodies, and `async` blocks.

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
