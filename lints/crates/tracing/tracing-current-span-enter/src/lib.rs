#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks re-entering the current span.
//!
//! This Dylint library resolves tracing span calls, reports redundant current
//! span entry, and recommends retaining one direct instrumentation operation.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
use tracing as _;

tracing_support::declare_direct_span_lint! {
    TRACING_CURRENT_SPAN_ENTER,
    TracingCurrentSpanEnter,
    method_on_direct_span,
    "tracing::span::Span::enter",
    ["tracing::span::Span::current"],
    "the current tracing span is re-entered",
    "`Span::current()` is already current",
    "remove the redundant enter guard"
}
