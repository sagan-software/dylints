# axum-route-legacy-wildcard-capture

## What it does

Checks for `axum::Router::route` and `route_service` calls whose path has a
segment that starts with `*`, the wildcard syntax from Axum 0.7 and earlier,
such as `"/assets/*path"`, including immutable local and `const` initializer
chains.

## Why is this bad?

Axum 0.8 writes wildcards as `{*name}`. By default, these methods panic on a
segment that starts with `*` when application code builds the router. Code that
still uses the old syntax after an upgrade fails when the application starts.

## Known problems

The lint follows at most eight immutable local or `const` initializer
references to a string literal. Mutable bindings, destructured bindings,
`static` values, function results, and runtime-built paths remain unknown. It
reports local or constant values at the call site and offers no
machine-applicable fix; fixes apply only to direct literals. The lint suppresses
this warning only when it can trace immutable router builders or local
bindings to `Router::without_v07_checks`. It can warn when that call is out of
sight, such as on a mutable binding or a router passed in as a parameter. It
does not check `nest` paths, where any wildcard panics. A machine-applicable
fix requires a plain string literal without escapes and Rust-style wildcard
identifiers.

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
