# axum-route-legacy-wildcard-capture

## What it does

Checks for `axum::Router::route` and `route_service` calls whose path has a
segment that starts with `*`, the wildcard syntax from Axum 0.7 and earlier,
such as `"/assets/*path"`.

## Why is this bad?

Axum 0.8 writes wildcards as `{*name}`. By default, these methods panic on a
segment that starts with `*` when application code builds the router. Code that
still uses the old syntax after an upgrade fails when the application starts.

## Known problems

The lint checks a string literal passed directly as the path, or a `const`
defined in the same crate and initialized with a string literal. It does not
check paths held in variables or built at runtime. The lint skips a router whose builder
chain or `let` initializer calls `Router::without_v07_checks`. It still warns
when that call is out of sight, such as on a router passed in as a parameter.
It does not check `nest` paths, where any wildcard panics. The
machine-applicable fix applies only to a plain string literal without escapes
whose wildcard names are Rust-style identifiers.

## Example

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("/assets/*path", get(handler))
}
```

## Use instead

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("/assets/{*path}", get(handler))
}
```
