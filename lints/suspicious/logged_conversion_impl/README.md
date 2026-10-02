# logged_conversion_impl

## What it does

Checks methods in `From`, `TryFrom`, and `FromStr` impls, including closures
inside them, for logging calls. Logging calls are `eprintln!`, the `debug!`,
`error!`, `event!`, `info!`, `log!`, `trace!`, and `warn!` macros of the `log`
and `tracing` crates, and functions such as `warn` or `log_error` that resolve
to a `kslog`, `log`, or `tracing` crate or module. A local macro that expands
to one of these logging macros also counts.

## Why is this bad?

Conversions run wherever a value is parsed or converted, including in loops
and in code that expects failure. A log line there repeats on every call, can
write raw input such as user data to logs, and takes the choice of log level
and wording away from the caller.

## Known problems

The lint reports only the first logging call in each method. It misses
logging done through helper functions, logging macros from other crates, and
logging in impls of other traits, such as `Into`. Conversion impls generated
by a macro are skipped, because the generated code cannot be edited where the
lint would point.

## Example

```rust
struct UserId(String);

impl From<String> for UserId {
    fn from(raw: String) -> Self {
        eprintln!("converting user id: {raw}");
        UserId(raw)
    }
}
```

## Use instead

```rust
struct UserId(String);

impl From<String> for UserId {
    fn from(raw: String) -> Self {
        UserId(raw)
    }
}
```
