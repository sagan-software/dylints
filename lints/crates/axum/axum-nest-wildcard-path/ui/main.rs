#![allow(dead_code)]

use axum::Router;

fn routes() {
    let _: Router = Router::new().nest("/api/{*rest}", Router::new());
    let _: Router = Router::new().nest("/api", Router::new());
}

fn main() {}
