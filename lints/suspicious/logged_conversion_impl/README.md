# logged_conversion_impl

## What it does

Checks methods in `From`, `TryFrom`, and `FromStr` impls for logging calls.
Logging calls are macros named `debug!`, `eprintln!`, `error!`, `info!`,
`trace!`, or `warn!`, and functions such as `log::warn(..)` or
`tracing::error(..)` from a `kslog`, `log`, or `tracing` path.

## Why is this bad?

Conversions run wherever a value is parsed or converted, including in loops
and in code that expects failure. A log line there repeats on every call, can
write raw input such as user data to logs, and takes the choice of log level
and wording away from the caller.

## Known problems

Macros are matched by name. A user-defined macro named `warn!` or `error!`
also triggers the lint. The method source text is also searched for these
macro names, so a match inside a comment, string, or closure, or a longer
name such as `my_warn!`, triggers the lint. In that case the whole method is
highlighted.

The lint reports only the first logging call in each method. It misses
logging done through helper functions, function calls inside closures, and
logging in impls of other traits, such as `Into`.

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
