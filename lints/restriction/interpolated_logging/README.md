# interpolated_logging

## What it does

Checks the message argument in macro calls whose final path segment is `trace`,
`debug`, `info`, `warn`, `error`, or `event`. It reports unescaped `{`
placeholders in cooked or raw string messages and literal `concat!` messages.
For literal `concat!` expressions, it recognizes strings, raw strings,
characters, booleans, integers, floats, and negative numeric literals. It reads
the `log` key/value `;` form and the positional level in `tracing::event!`,
while skipping recognized options and structured fields. Rust literal escapes are decoded
before the resulting message text is checked.

## Why is this bad?

A formatted value becomes part of free text. Log search and aggregation tools
cannot group records by that value. A structured field keeps the value under
its own key while the message stays constant.

## Known problems

The lint reads macro spelling before expansion and does not resolve macro
identity. A local macro whose final name matches a logging name can warn, while
an alias or wrapper with a different final name is not recognized. The lint
also reads `concat!` by spelling and applies the built-in literal rules, so a
local macro with that name can behave differently.

Only a direct string literal or literal `concat!` expression in the parsed
message position is checked. String values in recognized macro options and
structured fields are ignored. Computed message expressions, strings produced
by other macros, and strings added during logging-macro expansion are not
inspected. Formatted output such as `println!` and `write!` is not logging and
is not checked.

## Example

```rust
use tracing::info;

fn record_login(user_id: u64) {
    info!("user {user_id} logged in");
}
```

## Use instead

```rust
use tracing::info;

fn record_login(user_id: u64) {
    info!(user_id, "user logged in");
}
```
