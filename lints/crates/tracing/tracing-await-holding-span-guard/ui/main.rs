#![allow(dead_code)]

use tracing::{Instrument, info_span};

async fn other_work() {}

async fn invalid_borrowed_guard() {
    let span = info_span!("borrowed");
    let _guard = span.enter();
    other_work().await;
}

async fn invalid_owned_guard() {
    let _guard = info_span!("owned").entered();
    other_work().await;
}

async fn valid_scoped_work() {
    let span = info_span!("scoped");
    span.in_scope(|| {});
    other_work().await;
}

async fn valid_instrumented_work() {
    other_work().instrument(info_span!("instrumented")).await;
}

fn main() {}
