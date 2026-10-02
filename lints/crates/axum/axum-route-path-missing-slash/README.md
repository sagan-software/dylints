# axum-route-path-missing-slash

## What it does

Checks for `axum::Router::route` calls whose path is a nonempty string literal
that does not start with `/`, such as `"health"`.

## Why is this bad?

Axum route paths must start with `/`. `Router::route` panics on any other path
when the router is built, so the error shows up only when the application
starts or a test builds that router.

## Known problems

The lint checks only a string literal passed directly as the path. It does not
check paths held in constants or variables, or paths built at runtime. The
empty path `""` is reported by `axum-route-empty-path` instead.

## Example

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("health", get(handler))
}
```

## Use instead

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    Router::new().route("/health", get(handler))
}
```
