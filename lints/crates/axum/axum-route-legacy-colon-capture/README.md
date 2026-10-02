# axum-route-legacy-colon-capture

## What it does

Checks for `axum::Router::route` calls whose string literal path has a segment
that starts with `:`, the capture syntax from Axum 0.7 and earlier, such as
`"/users/:id"`.

## Why is this bad?

Axum 0.8 writes captures as `{name}`. By default, `Router::route` panics on a
segment that starts with `:` when the router is built. Code that still uses
the old syntax after an upgrade fails when the application starts.

## Known problems

The lint checks only a string literal passed directly as the path. It does not
check paths held in constants or variables, or paths built at runtime. It does
not check `Router::nest` or `Router::route_service` paths. It still warns when
the router calls `Router::without_v07_checks` to allow segments that start
with `:`.

## Example

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("/users/:id", get(handler))
}
```

## Use instead

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("/users/{id}", get(handler))
}
```
