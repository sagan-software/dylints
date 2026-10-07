#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks use of JSON snapshots when YAML gives more reliable diffs.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_discouraged_macro_lint! {
    INSTA_JSON_SNAPSHOT,
    InstaJsonSnapshot,
    "assert_json_snapshot",
    "an Insta JSON snapshot uses a format the guide normally discourages",
    "JSON snapshots make review diffs less reliable",
    "prefer `assert_yaml_snapshot!` unless exact JSON wire formatting is the subject"
}
