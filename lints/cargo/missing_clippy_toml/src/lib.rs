#![feature(rustc_private)]

//! A lint to check for missing clippy.toml files.
//!
//! It loads the package manifest for the current crate, finds the Cargo
//! workspace root, and checks the directories from the package up to that
//! root for a Clippy configuration file. A missing file is reported at the
//! package manifest's `[package]` header.

extern crate rustc_ast;
extern crate rustc_span;

use cargo_support::{emit_with_help, package_has_file, package_manifest};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub MISSING_CLIPPY_TOML,
    Warn,
    "crates should have a clippy.toml file",
    MissingClippyToml
}

/// Configuration file names Clippy reads from a directory.
const CONFIG_FILE_NAMES: &[&str] = &["clippy.toml", ".clippy.toml"];

impl EarlyLintPass for MissingClippyToml {
    /// Check that the crate's package or workspace has a Clippy configuration file.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(header) = missing_config_header(cx, krate) {
            emit_with_help(
                cx,
                MISSING_CLIPPY_TOML,
                header,
                "crate has no `clippy.toml` file",
                "add `clippy.toml` in the crate directory or workspace root",
            );
        }
    }
}

/// Return the `[package]` header span when no Clippy configuration applies to the package.
fn missing_config_header(cx: &EarlyContext<'_>, krate: &Crate) -> Option<Span> {
    let package = package_manifest(cx, krate)?;
    let header = package.entry_span("package")?;

    (!package_has_file(cx, &package, CONFIG_FILE_NAMES)).then_some(header)
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
