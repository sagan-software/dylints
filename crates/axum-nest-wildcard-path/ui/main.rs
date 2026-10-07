#![allow(dead_code)]

use axum::{Router, routing::get};

const WILDCARD: &str = "/files/{*rest}";

fn routes() {
    let _: Router = Router::new().nest("/api/{*rest}", Router::new());
    let _: Router = Router::new().nest("/api", Router::new());
    let _: Router = Router::new().nest_service("/static/{*path}", get(|| async {}));
    let _: Router = Router::new().nest(WILDCARD, Router::new());
}

fn escaped_braces_are_literal() {
    let _: Router = Router::new().nest("/api/{{*rest}}", Router::new());
    let _: Router = Router::new().nest("/api/{id}", Router::new());
}

fn main() {}

fn local_wildcard_path() {
    let path = "/api/{*rest}";
    let _: Router = Router::new().nest(path, Router::new());
}
