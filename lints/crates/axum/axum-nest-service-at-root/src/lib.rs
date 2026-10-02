#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Axum services nested at the root.
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
    AXUM_NEST_SERVICE_AT_ROOT,
    AxumNestServiceAtRoot,
    ["nest_service"],
    Root,
    "an Axum service is nested at the root",
    "nesting a service at the root panics",
    "install a root service with `Router::fallback_service`"
}
