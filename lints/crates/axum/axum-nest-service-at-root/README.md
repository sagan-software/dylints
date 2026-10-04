# axum-nest-service-at-root

## What it does

Checks for `axum::Router::nest_service` calls whose path resolves to `""` or
`"/"`, including immutable local and `const` initializer chains.

## Why is this bad?

Axum 0.8 does not support nesting a service at the root. `Router::nest_service`
panics with either path when application code builds the router, so the error
appears only when the application starts or a test builds that router.
`Router::fallback_service` sends every request that matches no route to the
service.

## Known problems

The lint follows at most eight immutable local or `const` initializer
references to a string literal. Mutable bindings, destructured bindings,
`static` values, function results, and runtime-built paths remain unknown. The
lint reports local or constant values at the call site and offers no
machine-applicable fix; fixes apply only to direct literals.

## Example

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().nest_service("/", api)
}
```

## Use instead

```rust
use axum::Router;

fn app(api: Router) -> Router {
    Router::new().fallback_service(api)
}
```
