# axum-route-layer-on-empty-router

## What it does

Checks for `axum::Router::route_layer` called on a router that has no routes
yet. The lint follows the receiver back through `let` bindings and route-free
builder calls, such as `layer`, `fallback`, and `with_state`, to
`Router::new()`.

## Why is this bad?

`Router::route_layer` wraps only the routes that already exist. On a router
with no routes it would do nothing, so Axum panics when the router is built.
The error shows up only when the application starts or a test builds that
router.

## Known problems

The lint does not follow an empty router through a helper function, a function
parameter, or `Router::merge`. It does not check `Router::default()`.

## Example

```rust
use axum::{
    extract::Request,
    middleware::{self, Next},
    response::Response,
    Router,
};

async fn auth(request: Request, next: Next) -> Response {
    next.run(request).await
}

fn app() -> Router {
    Router::new().route_layer(middleware::from_fn(auth))
}
```

## Use instead

Add the routes before the route layer. Use `Router::layer` instead if the layer
must also wrap the fallback.

```rust
use axum::{
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};

async fn auth(request: Request, next: Next) -> Response {
    next.run(request).await
}

async fn handler() {}

fn app() -> Router {
    Router::new()
        .route("/", get(handler))
        .route_layer(middleware::from_fn(auth))
}
```
