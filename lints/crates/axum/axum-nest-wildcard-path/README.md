# axum-nest-wildcard-path

## What it does

Checks for `axum::Router::nest` calls whose string literal path contains a
wildcard capture such as `{*rest}`.

## Why is this bad?

Axum does not allow wildcard captures in the path of a nested router.
`Router::nest` panics when the router is built, so the error shows up only when
the application starts or a test builds that router. The wildcard route belongs
inside the nested router.

## Known problems

The lint checks only a string literal passed directly as the path. It does not
check paths held in constants or variables, or paths built at runtime. It does
not check `Router::nest_service` paths.

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
