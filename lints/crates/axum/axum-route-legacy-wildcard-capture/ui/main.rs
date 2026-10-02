// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use axum::{Router, routing::get};

fn routes() {
    let _: Router = Router::new().route("/assets/*path", get(|| async {}));
    let _: Router = Router::new().route("/assets/{*path}", get(|| async {}));
    let _: Router = Router::new().route_service("/files/*rest", get(|| async {}));
}

fn checks_disabled() {
    let mut router: Router = Router::new().without_v07_checks();
    router = router.route("/literal/*star", get(|| async {}));
    let _ = router;
}

fn main() {}
