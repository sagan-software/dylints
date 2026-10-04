// compile-flags: -Copt-level=3
//! Optimized compiler UI cases for tracing guard ownership.
#![feature(rustc_private)]
#![allow(
    dead_code,
    reason = "UI functions are compiled to inspect diagnostics and are not called."
)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use tracing::Span;
use tracing_await_holding_span_guard as _;

/// Provide a small await point for each optimized UI case.
async fn other_work() {
    std::future::ready(()).await;
}

/// Keep an optional guard across an await point.
async fn wrapped_guard(span: Span) {
    let _guard = Some(span.entered());
    other_work().await;
}

/// Stay quiet when the option contains no guard.
async fn none_guard() {
    let _guard: Option<tracing::span::EnteredSpan> = None;
    other_work().await;
}

/// Stay quiet when the wrapper is dropped before suspension.
async fn dropped_guard(span: Span) {
    let guard = Some(span.entered());
    drop(guard);
    other_work().await;
}

/// Stay quiet when `Option::take` removes and drops the guard.
async fn taken_and_dropped_guard(span: Span) {
    let mut guard = Some(span.entered());
    drop(guard.take());
    other_work().await;
}

/// Stay quiet when each branch releases the wrapper.
async fn released_on_both_branches(span: Span, first_branch: bool) {
    let guard = Some(span.entered());
    if first_branch {
        drop(guard);
    } else {
        drop(Some(guard));
    }
    other_work().await;
}

/// Warn when one branch retains the wrapper through suspension.
async fn retained_on_one_branch(span: Span, release: bool) {
    let guard = Some(span.entered());
    if release {
        drop(guard);
    }
    other_work().await;
}

/// Warn when a `Box` retains the guard.
async fn boxed_guard(span: Span) {
    let guard = Box::new(span.entered());
    other_work().await;
    drop(guard);
}

/// Store an optional guard in a wrapper struct.
struct GuardWrapper {
    /// Optional guard retained by the struct.
    guard: Option<tracing::span::EnteredSpan>,
}

/// Warn when the struct retains the guard at optimized suspension points.
async fn struct_guard(span: Span) {
    let guard = GuardWrapper {
        guard: Some(span.entered()),
    };
    other_work().await;
    drop(guard);
}

/// Warn when a tuple retains the guard at optimized suspension points.
async fn tuple_guard(span: Span) {
    let guard = (span.entered(),);
    other_work().await;
    drop(guard);
}

/// Warn when `Box<Option<_>>` retains the guard across optimized suspension.
async fn boxed_option_guard(span: Span) {
    let guard = Box::new(Some(span.entered()));
    other_work().await;
    drop(guard);
}

/// Stay quiet when the boxed option is dropped before optimized suspension.
async fn dropped_boxed_option_guard(span: Span) {
    let guard = Box::new(Some(span.entered()));
    drop(guard);
    other_work().await;
}

/// Warn when a boxed user-defined wrapper retains the guard across suspension.
async fn boxed_struct_guard(span: Span) {
    let guard = Box::new(GuardWrapper {
        guard: Some(span.entered()),
    });
    other_work().await;
    drop(guard);
}

/// Stay quiet when the boxed wrapper is dropped before optimized suspension.
async fn dropped_boxed_struct_guard(span: Span) {
    let guard = Box::new(GuardWrapper {
        guard: Some(span.entered()),
    });
    drop(guard);
    other_work().await;
}

/// Satisfy the optimized compiler UI fixture entry-point requirement.
fn main() {}
