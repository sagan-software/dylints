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

fn valid_after_route() {
    let _: Router = Router::new()
        .route("/", get(|| async {}))
        .route_layer(middleware::from_fn(pass_through));
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
