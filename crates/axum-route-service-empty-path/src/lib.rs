#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for empty Axum route-service paths.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
use rustc_lint::LintContext as _;

#[cfg(test)]
use axum as _;

axum_support::declare_router_path_lint! {
    AXUM_ROUTE_SERVICE_EMPTY_PATH,
    AxumRouteServiceEmptyPath,
    ["route_service"],
    Empty,
    "an Axum route service has an empty path",
    "this empty route-service path panics",
    "use a nonempty path beginning with `/`"
}
