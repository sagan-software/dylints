#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty Insta snapshot input files.
//!
//! This Dylint library resolves Insta snapshot settings, reports empty input
//! files, and recommends reviewer context or omitting the empty setting.
//!
//! The README defines the supported settings and replacement. UI fixtures cover
//! triggering and non-triggering forms so unrelated snapshot code remains unchanged.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_empty_settings_lint! {
    INSTA_EMPTY_INPUT_FILE,
    InstaEmptyInputFile,
    "set_input_file",
    "an Insta snapshot input file is empty",
    "provide reviewer context or omit the setting"
}
