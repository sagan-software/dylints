#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for empty Axum route paths.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Axum route literals, reports empty paths, and
//! recommends an explicit root or non-empty route path.

extern crate rustc_errors;
extern crate rustc_hir;
use rustc_lint::LintContext as _;

#[cfg(test)]
use axum as _;

axum_support::declare_router_path_lint! {
    AXUM_ROUTE_EMPTY_PATH,
    AxumRouteEmptyPath,
    "route",
    Empty,
    "an Axum route has an empty path",
    "this empty route path panics",
    "use `/` for the root route"
}
