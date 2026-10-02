// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use axum::{Router, routing::get};

const RELATIVE: &str = "relative";
const ALIAS: &str = RELATIVE;
static STATIC_PATH: &str = "static";

fn routes() {
    let _: Router = Router::new().route("health", get(|| async {}));
    let _: Router = Router::new().route("/health", get(|| async {}));
    let _: Router = Router::new().route_service("files", get(|| async {}));
    let _: Router = Router::new().nest("api", Router::new());
    let _: Router = Router::new().nest_service("assets", get(|| async {}));
    let _: Router = Router::new().route(RELATIVE, get(|| async {}));
}

fn not_literal_paths(path: String) {
    let _: Router = Router::new().route(ALIAS, get(|| async {}));
    let _: Router = Router::new().route(STATIC_PATH, get(|| async {}));
    let _: Router = Router::new().route(&path, get(|| async {}));
}

fn main() {}
