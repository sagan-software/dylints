#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks explicit current-span instrumentation.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
use tracing as _;

tracing_support::declare_direct_span_lint! {
    TRACING_INSTRUMENT_CURRENT_SPAN,
    TracingInstrumentCurrentSpan,
    method_with_direct_span_argument,
    "tracing::instrument::Instrument::instrument",
    ["tracing::span::Span::current"],
    "a future is instrumented with Span::current",
    "`Span::current()` is an indirect spelling of current-span instrumentation",
    "call `.in_current_span()`"
}
