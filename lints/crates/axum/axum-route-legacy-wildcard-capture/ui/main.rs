#![allow(dead_code)]

use axum::{Router, routing::get};

fn routes() {
    let _: Router = Router::new().route("/assets/*path", get(|| async {}));
    let _: Router = Router::new().route("/assets/{*path}", get(|| async {}));
}

fn main() {}
