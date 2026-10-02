#![allow(dead_code)]

use tracing::{Instrument, info_span};

async fn work() {}

async fn invalid_nested_instrumentation() {
    work()
        .instrument(info_span!("work"))
        .in_current_span()
        .await;
}

async fn valid_or_current() {
    work().instrument(info_span!("work").or_current()).await;
}

struct OtherFuture;

impl OtherFuture {
    fn instrument(self, _span: tracing::Span) -> Self {
        self
    }

    fn in_current_span(self) -> Self {
        self
    }
}

fn similarly_named_user_methods() {
    let _future = OtherFuture
        .instrument(info_span!("other"))
        .in_current_span();
}

fn main() {}
