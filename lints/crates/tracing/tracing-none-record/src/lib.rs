#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks recording fields on a disabled span.
//!
//! This Dylint library resolves tracing span calls, reports fields recorded on
//! disabled spans, and recommends keeping recording on an active span.
//!
//! The README defines the supported call shapes and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
use tracing as _;

tracing_support::declare_direct_span_lint! {
    TRACING_NONE_RECORD,
    TracingNoneRecord,
    method_on_direct_span,
    "tracing::span::Span::record",
    ["tracing::span::Span::none"],
    "a field is recorded on a disabled tracing span",
    "recording on `Span::none()` has no effect",
    "remove the record or create an enabled span"
}
