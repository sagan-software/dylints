# axum-nest-service-at-root

## What it does

Checks for `axum::Router::nest_service` calls whose path is the string literal
`""` or `"/"`.

## Why is this bad?

Axum 0.8 does not support nesting a service at the root. `Router::nest_service`
panics with either path when the router is built, so the error shows up only
when the application starts or a test builds that router.
`Router::fallback_service` sends every request that matches no route to the
service.

## Known problems

The lint checks a string literal passed directly as the path, or a `const`
defined in the same crate and initialized with a string literal. It does not
check paths held in variables or built at runtime.

## Example

```rust
use axum::Router;
use tower_http::services::ServeDir;

fn app() -> Router {
    Router::new().nest_service("/", ServeDir::new("assets"))
}
```

## Use instead

```rust
use axum::Router;
use tower_http::services::ServeDir;

fn app() -> Router {
    Router::new().fallback_service(ServeDir::new("assets"))
}
```
