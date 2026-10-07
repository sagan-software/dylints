#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `or_current` on the current span.
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
    TRACING_CURRENT_SPAN_OR_CURRENT,
    TracingCurrentSpanOrCurrent,
    method_on_direct_span,
    "tracing::span::Span::or_current",
    ["tracing::span::Span::current"],
    "Span::current is followed by or_current",
    "`or_current` is redundant on `Span::current()`",
    "remove `.or_current()`"
}
