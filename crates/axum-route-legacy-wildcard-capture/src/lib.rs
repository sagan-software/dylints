#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Axum 0.7 wildcard captures.
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
    AXUM_ROUTE_LEGACY_WILDCARD_CAPTURE,
    AxumRouteLegacyWildcardCapture,
    ["route", "route_service"],
    LegacyWildcardCapture,
    "an Axum route uses a legacy wildcard capture",
    "Axum 0.8 rejects this legacy `*name` capture",
    "write the wildcard as `{*name}`"
}
