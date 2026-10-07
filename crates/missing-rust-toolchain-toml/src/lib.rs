#![feature(rustc_private)]

//! A lint to check for missing rust-toolchain.toml files.
//!
//! It loads the package manifest for the current crate, finds the Cargo
//! workspace root, and checks the directories from the package up to that
//! root for a `rust-toolchain.toml` file. A missing file is reported at the
//! package manifest's `[package]` header.

extern crate rustc_ast;
extern crate rustc_span;

use cargo_support::{emit_with_help, package_has_file, package_manifest};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub MISSING_RUST_TOOLCHAIN_TOML,
    Warn,
    "crates should have a rust-toolchain.toml file",
    MissingRustToolchainToml
}

/// Toolchain file name this lint requires.
const TOOLCHAIN_FILE_NAMES: &[&str] = &["rust-toolchain.toml"];

impl EarlyLintPass for MissingRustToolchainToml {
    /// Check that the crate's package or workspace pins a toolchain.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(header) = missing_toolchain_header(cx, krate) {
            emit_with_help(
                cx,
                MISSING_RUST_TOOLCHAIN_TOML,
                header,
                "crate has no `rust-toolchain.toml` file",
                "add `rust-toolchain.toml` in the crate directory or workspace root",
            );
        }
    }
}

/// Return the `[package]` header span when no toolchain file applies to the package.
fn missing_toolchain_header(cx: &EarlyContext<'_>, krate: &Crate) -> Option<Span> {
    let package = package_manifest(cx, krate)?;
    let header = package.entry_span("package")?;

    (!package_has_file(cx, &package, TOOLCHAIN_FILE_NAMES)).then_some(header)
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
