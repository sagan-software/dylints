#![allow(dead_code)]

use tracing::{Instrument, info_span, subscriber};

async fn work() {}

async fn invalid_span_scope() {
    let span = info_span!("span_scope");
    let future = span.in_scope(|| async { work().await });
    future.await;
}

async fn invalid_subscriber_scope() {
    let future = subscriber::with_default(subscriber::NoSubscriber::default(), || async {
        work().await;
    });
    future.await;
}

async fn valid_instrumented() {
    work().instrument(info_span!("instrumented")).await;
}

fn valid_synchronous_scope() {
    let span = info_span!("synchronous");
    span.in_scope(|| {});
}

fn main() {}
