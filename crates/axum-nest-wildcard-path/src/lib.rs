#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for wildcard captures in Axum nest paths.
//!
//! This Dylint library resolves Axum nest route literals, reports unsupported
//! wildcard captures, and recommends a route shape supported by the nest API.
//!
//! The README defines the supported call shapes and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
use rustc_lint::LintContext as _;

#[cfg(test)]
use axum as _;

axum_support::declare_router_path_lint! {
    AXUM_NEST_WILDCARD_PATH,
    AxumNestWildcardPath,
    ["nest", "nest_service"],
    NestedWildcard,
    "an Axum nest path contains a wildcard",
    "wildcards in nested router paths panic",
    "nest at a fixed prefix and route the wildcard inside the nested router"
}
