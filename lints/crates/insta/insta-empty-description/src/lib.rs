#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty Insta snapshot descriptions.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Insta settings calls, reports empty snapshot
//! descriptions, and recommends a meaningful description or omitted setting.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_empty_settings_lint! {
    INSTA_EMPTY_DESCRIPTION,
    InstaEmptyDescription,
    "set_description",
    "an Insta snapshot description is empty",
    "provide reviewer context or omit the setting"
}
