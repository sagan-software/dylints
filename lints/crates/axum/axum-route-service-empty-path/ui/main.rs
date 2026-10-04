// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use axum::Router;

fn routes() {
    let _: Router = Router::new().route_service("", Router::new());
}

fn main() {}

fn local_empty_path() {
    let path = "";
    let _: Router = Router::new().route_service(path, Router::new());
    let _: Router = Router::new().route_service(path, Router::new());
}
