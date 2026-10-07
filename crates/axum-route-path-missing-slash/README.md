# axum-route-path-missing-slash

## What it does

Checks for `axum::Router::route`, `route_service`, `nest`, and `nest_service`
calls whose path is a nonempty string that does not start with `/`, such as
`"health"`, including immutable local and `const` initializer chains.

## Why is this bad?

Axum paths must start with `/`. These methods panic on any other path when
application code builds the router. The error appears only when the
application starts or a test builds that router.

## Known problems

The lint follows at most eight immutable local or `const` initializer
references to a string literal. Mutable bindings, destructured bindings,
`static` values, function results, and runtime-built paths remain unknown. It
reports local or constant values at the call site and leaves their initializers
unchanged. Machine-applicable fixes apply only to direct string literals without
escapes.
The other lints report the empty path `""`:
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
