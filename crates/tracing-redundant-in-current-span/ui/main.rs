//! UI examples for `tracing-redundant-in-current-span`.

#![cfg_attr(clippy, feature(rustc_private))]

#[cfg(clippy)]
use dylint_linting as _;
#[cfg(clippy)]
use dylint_support as _;
#[cfg(clippy)]
use dylint_testing as _;
use tracing::{Instrument, info_span};
#[cfg(clippy)]
use tracing_redundant_in_current_span as _;
#[cfg(clippy)]
use tracing_subscriber as _;
#[cfg(clippy)]
use tracing_support as _;

/// Return a future for the instrumentation examples.
async fn work() {
    std::future::ready(()).await;
}

/// Trigger the lint on directly nested tracing instrumentation.
async fn invalid_nested_instrumentation() {
    work()
        .instrument(info_span!("work"))
        .in_current_span()
        .await;
}

/// Trigger the lint with a parentless inner span and an entered current span.
fn independent_inner_span() {
    let _outer = info_span!("outer").entered();
    let _future = work()
        .instrument(info_span!(parent: None, "inner"))
        .in_current_span();
}

/// Show the supported `or_current` form.
async fn valid_or_current() {
    work().instrument(info_span!("work").or_current()).await;
}

/// Show that `in_current_span` without an inner `instrument` call is outside this lint.
async fn current_span_without_inner_instrumentation() {
    work().in_current_span().await;
}

/// Provide methods that resemble `Instrument` without implementing that trait.
struct OtherFuture;

impl OtherFuture {
    /// Return this value from an unrelated `instrument` method.
    fn instrument(self, _span: tracing::Span) -> Self {
        self
    }

    /// Return this value from an unrelated `in_current_span` method.
    const fn in_current_span(self) -> Self {
        self
    }
}

/// Keep tracing-like user methods quiet.
fn similarly_named_user_methods() {
    let _future = OtherFuture
        .instrument(info_span!("other"))
        .in_current_span();
}

fn main() {
    drop(invalid_nested_instrumentation());
    independent_inner_span();
    drop(valid_or_current());
    drop(current_span_without_inner_instrumentation());
    similarly_named_user_methods();
}
