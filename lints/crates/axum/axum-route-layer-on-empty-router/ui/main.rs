#![allow(dead_code)]

use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
};

async fn pass_through(request: Request, next: Next) -> Response {
    next.run(request).await
}

fn invalid() {
    let _: Router = Router::new().route_layer(middleware::from_fn(pass_through));
}

fn invalid_after_route_free_calls() {
    let _: Router = Router::new()
        .layer(middleware::from_fn(pass_through))
        .fallback(|| async {})
        .route_layer(middleware::from_fn(pass_through));
    let router = Router::new();
    let _: Router = router.route_layer(middleware::from_fn(pass_through));
}

fn valid_after_route() {
    let _: Router = Router::new()
        .route("/", get(|| async {}))
        .route_layer(middleware::from_fn(pass_through));
    let router = Router::new().route("/", get(|| async {}));
    let _: Router = router.route_layer(middleware::from_fn(pass_through));
}

fn valid_unknown_router(router: Router) -> Router {
    router.route_layer(middleware::from_fn(pass_through))
}

fn valid_called_constructors() {
    let make = Router::new;
    let _: Router = make().route_layer(middleware::from_fn(pass_through));
    let _: Router = (|| Router::new())().route_layer(middleware::from_fn(pass_through));
}

fn valid_merged_router(other: Router) -> Router {
    Router::new()
        .merge(other)
        .route_layer(middleware::from_fn(pass_through))
}

struct OtherRouter;

impl OtherRouter {
    fn new() -> Self {
        Self
    }

    fn route_layer(self, _layer: ()) -> Self {
        self
    }
}

fn same_names_are_not_axum() {
    let _router = OtherRouter::new().route_layer(());
}

fn main() {}
