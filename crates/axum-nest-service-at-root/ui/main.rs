#![allow(dead_code)]

use axum::Router;

fn routes() {
    let _: Router = Router::new().nest_service("/", Router::new());
}

fn main() {}

fn local_root_path() {
    let path = "/";
    let _: Router = Router::new().nest_service(path, Router::new());
}
