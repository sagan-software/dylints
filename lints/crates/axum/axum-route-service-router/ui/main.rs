#![allow(dead_code)]

use axum::{Router, routing::get};

fn invalid() {
    let inner = Router::new().route("/", get(|| async {}));
    let _: Router = Router::new().route_service("/api", inner);
}

fn valid_nested_router() {
    let inner = Router::new().route("/", get(|| async {}));
    let _: Router = Router::new().nest("/api", inner);
}

struct OtherRouter;

impl OtherRouter {
    fn route_service(self, _path: &str, _service: OtherRouter) -> Self {
        self
    }
}

fn same_names_are_not_axum() {
    let _router = OtherRouter.route_service("/api", OtherRouter);
}

fn main() {}
