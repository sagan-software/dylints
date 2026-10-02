#![feature(rustc_private)]

//! A lint to check for package manifests without a lints section.
//!
//! It parses the nearest `Cargo.toml` above the crate root with `toml_edit`.
//! When that manifest has a `[package]` table and no top-level `lints` key in
//! any TOML form, the lint reports the `[package]` header. Virtual workspace
//! manifests have no `[package]` table and are skipped.

extern crate rustc_ast;
extern crate rustc_span;

use cargo_support::{emit_with_help, package_manifest};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub PACKAGE_LINTS_SECTION,
    Warn,
    "package manifests should define a `[lints]` table",
    PackageLintsSection
}

impl EarlyLintPass for PackageLintsSection {
    /// Check that the crate's package manifest configures lints.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(header) = missing_lints_header(cx, krate) {
            emit_with_help(
                cx,
                PACKAGE_LINTS_SECTION,
                header,
                "package `Cargo.toml` defines `[package]` but no `[lints]` table",
                "add a package `[lints]` table, usually with `workspace = true`",
            );
        }
    }
}

/// Return the `[package]` header span when the package manifest has no `lints` key.
fn missing_lints_header(cx: &EarlyContext<'_>, krate: &Crate) -> Option<Span> {
    let package = package_manifest(cx, krate)?;
    let header = package.entry_span("package")?;

    // A `[lints]` table, a dotted `lints.workspace` key, and an inline table all count.
    (!package.root().contains_key("lints")).then_some(header)
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
