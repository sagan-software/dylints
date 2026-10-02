# axum-route-service-empty-path

## What it does

Checks for `axum::Router::route_service` calls whose path is the empty string
literal `""`.

## Why is this bad?

Axum route paths must start with `/`. `Router::route_service` panics on an
empty path when application code builds the router. The error appears only when
the application starts or a test builds that router. The root route is `"/"`.

## Known problems

The lint checks a string literal passed directly as the path, or a `const`
defined in the same crate and initialized with a string literal. It does not
check paths held in variables or built at runtime.

## Example

```rust
use axum::Router;
use tower_http::services::ServeFile;

fn app() -> Router {
    Router::new().route_service("", ServeFile::new("index.html"))
}
```

## Use instead

```rust
use axum::Router;
use tower_http::services::ServeFile;

fn app() -> Router {
    Router::new().route_service("/", ServeFile::new("index.html"))
}
```
