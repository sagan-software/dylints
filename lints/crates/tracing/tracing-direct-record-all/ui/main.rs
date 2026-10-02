#![allow(dead_code)]

use tracing::{field, info_span};

fn invalid_direct_call() {
    let span = info_span!("direct", answer = field::Empty);
    if let Some(metadata) = span.metadata() {
        span.record_all(&tracing::valueset!(metadata.fields(), answer = 42));
    }
}

fn valid_macro_call() {
    let span = info_span!("macro", answer = field::Empty);
    tracing::record_all!(span, answer = 42);
}

struct OtherSpan;

impl OtherSpan {
    fn record_all(&self, _values: &[u8]) {}
}

fn similarly_named_user_method() {
    OtherSpan.record_all(&[]);
}

fn main() {}
