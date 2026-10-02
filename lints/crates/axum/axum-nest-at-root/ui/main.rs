#![allow(dead_code)]

use axum::Router;

fn routes() {
    let _: Router = Router::new().nest("/", Router::new());
    let _: Router = Router::new().nest("/api", Router::new());
}

fn main() {}
