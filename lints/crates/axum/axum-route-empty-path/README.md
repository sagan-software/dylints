# axum-route-empty-path

## What it does

Checks for `axum::Router::route` calls whose path resolves to the empty string,
including immutable local and `const` initializer chains.

## Why is this bad?

Axum route paths must start with `/`. `Router::route` panics on an empty path
when application code builds the router. The error appears only when the
application starts or a test builds that router. The root route is `"/"`.

## Known problems

The lint follows at most eight immutable local or `const` initializer
references to a string literal. Mutable bindings, destructured bindings,
`static` values, function results, and runtime-built paths remain unknown. The
lint reports local or constant values at the call site and offers no
machine-applicable fix; fixes apply only to direct literals.

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
