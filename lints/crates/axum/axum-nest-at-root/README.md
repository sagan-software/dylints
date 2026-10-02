# axum-nest-at-root

## What it does

Checks for `axum::Router::nest` calls whose path is the string literal `""` or
`"/"`.

## Why is this bad?

Axum 0.8 does not support nesting a router at the root. `Router::nest` panics
with either path when the router is built, so the error shows up only when the
application starts or a test builds that router. `Router::merge` combines two
routers at the same level.

## Known problems

The lint checks only a string literal passed directly as the path. It does not
check paths held in constants or variables, or paths built at runtime.

## Example

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().nest("/", api)
}
```

## Use instead

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().merge(api)
}
```
