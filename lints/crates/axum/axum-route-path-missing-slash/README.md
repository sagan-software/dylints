# axum-route-path-missing-slash

## What it does

Checks for `axum::Router::route`, `route_service`, `nest`, and `nest_service`
calls whose path is a nonempty string that does not start with `/`, such as
`"health"`.

## Why is this bad?

Axum paths must start with `/`. These methods panic on any other path when the
router is built, so the error shows up only when the application starts or a
test builds that router.

## Known problems

The lint checks a string literal passed directly as the path, or a `const`
defined in the same crate and initialized with a string literal. It does not
check paths held in variables or built at runtime. The machine-applicable fix applies only
to a plain string literal without escapes. The empty path `""` is reported by
`axum-route-empty-path`, `axum-route-service-empty-path`, `axum-nest-at-root`,
or `axum-nest-service-at-root` instead.

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
