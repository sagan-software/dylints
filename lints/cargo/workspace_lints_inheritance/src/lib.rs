#![feature(rustc_private)]

//! A lint to check for workspace packages that do not inherit workspace lints.
//!
//! It parses the nearest package manifest above the crate root with
//! `toml_edit`. When the package defines `lints` without `workspace = true`
//! and its Cargo workspace root declares `[workspace.lints]`, the lint reports
//! the package's `lints` key or table header.

extern crate rustc_ast;
extern crate rustc_span;

use cargo_support::{emit_with_help, has_workspace_entry, is_true, package_manifest};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub WORKSPACE_LINTS_INHERITANCE,
    Warn,
    "workspace package lints should inherit from workspace lint settings",
    WorkspaceLintsInheritance
}

impl EarlyLintPass for WorkspaceLintsInheritance {
    /// Check that a workspace package's `lints` table inherits the workspace lints.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(lints) = non_inheriting_lints(cx, krate) {
            emit_with_help(
                cx,
                WORKSPACE_LINTS_INHERITANCE,
                lints,
                "workspace package defines `[lints]` without `workspace = true`",
                "add `workspace = true` to the package `[lints]` table",
            );
        }
    }
}

/// Return the span of a package `lints` entry that should inherit the workspace lints.
fn non_inheriting_lints(cx: &EarlyContext<'_>, krate: &Crate) -> Option<Span> {
    let package = package_manifest(cx, krate)?;
    let lints = package.root().get("lints")?;

    // A `workspace = true` entry already inherits the workspace lints.
    if lints
        .as_table_like()
        .is_some_and(|table| is_true(table.get("workspace")))
    {
        return None;
    }

    // `workspace = true` is only valid when the workspace root declares `[workspace.lints]`.
    if !has_workspace_entry(cx, &package, "lints") {
        return None;
    }

    package.entry_span("lints")
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
