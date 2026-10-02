# axum-nest-wildcard-path

## What it does

Checks for `axum::Router::nest` and `axum::Router::nest_service` calls whose
path has a wildcard capture segment such as `{*rest}`.

## Why is this bad?

Axum does not allow wildcard captures in the path of a nested router.
`Router::nest` panics when application code builds the router, so the error
appears only when the application starts or a test builds that router. The
wildcard route belongs inside the nested router.

## Known problems

The lint checks a string literal passed directly as the path, or a `const`
defined in the same crate and initialized with a string literal. It does not
check paths held in variables or built at runtime. An escaped segment such as
`{{*rest}}` is a literal path, so the lint does not report it.

## Example

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().nest("/api/{*rest}", api)
}
```

## Use instead

```rust
use axum::{routing::get, Router};

async fn handler() {}

fn app() -> Router {
    let api = Router::new().route("/{*rest}", get(handler));
    Router::new().nest("/api", api)
}
```
