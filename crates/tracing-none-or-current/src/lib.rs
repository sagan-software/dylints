#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `or_current` on a disabled span.
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
    TRACING_NONE_OR_CURRENT,
    TracingNoneOrCurrent,
    method_on_direct_span,
    "tracing::span::Span::or_current",
    ["tracing::span::Span::none"],
    "Span::none is followed by or_current",
    "`Span::none().or_current()` always selects the current span",
    "use `Span::current()`"
}
