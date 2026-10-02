# axum-route-service-router

## What it does

Checks for `axum::Router::route_service` calls whose service argument is an
`axum::Router`.

## Why is this bad?

`Router::route_service` panics when the service is another `Router`, so the
error shows up only when the application starts or a test builds that router.
`Router::nest` mounts a router below a path prefix, and `Router::merge` combines
two routers at the same level.

## Known problems

The lint checks the type of the service argument. It does not detect a router
wrapped in another service type, such as a router passed through a Tower
`ServiceBuilder`.

## Example

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().route_service("/api", api)
}
```

## Use instead

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().nest("/api", api)
}
```
