// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use axum::{Router, routing::get};

fn routes() {
    let _: Router = Router::new().route("/users/:id", get(|| async {}));
    let _: Router = Router::new().route("/users/{id}", get(|| async {}));
}

fn main() {}
