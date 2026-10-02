#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks compact JSON snapshots that trade away line-oriented diffs.
//!
//! This Dylint library resolves Insta snapshot macros, reports compact JSON
//! where line-oriented review matters, and recommends a readable snapshot.
//!
//! The README defines the supported snapshot shapes and replacement. UI
//! fixtures cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_discouraged_macro_lint! {
    INSTA_COMPACT_JSON_SNAPSHOT,
    InstaCompactJsonSnapshot,
    "assert_compact_json_snapshot",
    "an Insta compact JSON snapshot reduces diff readability",
    "compact JSON snapshots produce worse line-oriented diffs",
    "prefer `assert_yaml_snapshot!` unless single-line JSON is the tested contract"
}
