#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Axum 0.7 colon captures.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Axum route literals, reports legacy colon
//! captures, and recommends the current brace capture syntax.

extern crate rustc_errors;
extern crate rustc_hir;
use rustc_lint::LintContext as _;

#[cfg(test)]
use axum as _;

axum_support::declare_router_path_lint! {
    AXUM_ROUTE_LEGACY_COLON_CAPTURE,
    AxumRouteLegacyColonCapture,
    "route",
    LegacyColonCapture,
    "an Axum route uses a legacy colon capture",
    "Axum 0.8 rejects this legacy `:name` capture",
    "write the capture as `{name}`"
}
