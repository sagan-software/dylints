# axum-route-empty-path

## What it does

Checks for `axum::Router::route` calls whose path is the empty string literal
`""`.

## Why is this bad?

Axum route paths must start with `/`. `Router::route` panics on an empty path
when the router is built, so the error shows up only when the application
starts or a test builds that router. The root route is `"/"`.

## Known problems

The lint checks only a string literal passed directly as the path. It does not
check paths held in constants or variables, or paths built at runtime.

## Example

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("", get(handler))
}
```

## Use instead

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("/", get(handler))
}
```
