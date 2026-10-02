# interpolated_logging

## What it does

Checks for logging macro calls with a string literal argument that contains a
format placeholder such as `{}` or `{user_id}`. The macro names are `trace`,
`debug`, `info`, `warn`, `error`, and `event`.

## Why is this bad?

A value formatted into the message becomes part of free text. Log search and
aggregation tools cannot filter or group by it, and each distinct value makes a
distinct message. A structured field keeps the value under its own key and the
message constant.

## Known problems

The lint matches macros by the last segment of their name before expansion, so
`log::info!`, `tracing::info!`, and a local `info!` macro all warn. A wrapper
macro with another name does not warn.

It does not check formatted output such as `println!` or
`write!(f, "{}", self.0)` in a `Display` impl. It warns when any string argument
of a logging macro contains an unescaped `{`, such as a JSON field value.

## Example

```rust
use tracing::{info, warn};

fn record_login(user_id: u64, request_id: &str) {
    info!("user {} logged in", user_id);
    warn!("request failed for {request_id}");
}
```

## Use instead

```rust
use tracing::{info, warn};

fn record_login(user_id: u64, request_id: &str) {
    info!(user_id, "user logged in");
    warn!(request_id, "request failed");
}
```
