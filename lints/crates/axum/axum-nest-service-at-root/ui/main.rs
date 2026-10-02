#![allow(dead_code)]

use axum::Router;

fn routes() {
    let _: Router = Router::new().nest_service("/", Router::new());
}

fn main() {}
