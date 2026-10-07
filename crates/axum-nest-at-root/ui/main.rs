#![allow(dead_code)]

use axum::Router;

fn routes() {
    let _: Router = Router::new().nest("/", Router::new());
    let _: Router = Router::new().nest("/api", Router::new());
}

fn main() {}

fn local_root_path() {
    let path = "/";
    let _: Router = Router::new().nest(path, Router::new());
}
