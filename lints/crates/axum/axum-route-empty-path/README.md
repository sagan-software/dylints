# axum-route-empty-path

## What it does

Checks for `axum::Router::route` calls whose path is the empty string literal
`""`.

## Why is this bad?

Axum route paths must start with `/`. `Router::route` panics on an empty path
when application code builds the router. The error appears only when the
application starts or a test builds that router. The root route is `"/"`.

## Known problems

The lint checks a string literal passed directly as the path, or a `const`
defined in the same crate and initialized with a string literal. It does not
check paths held in variables or built at runtime.

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
