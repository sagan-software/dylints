#![feature(rustc_private)]

//! A lint to check for workspace packages with local dependency versions.
//!
//! It parses the nearest package manifest above the crate root with
//! `toml_edit`, finds its Cargo workspace root, and checks the package's
//! dependency tables only when that root declares `[workspace.dependencies]`.
//! Each dependency with its own `version` is reported at that version value.

extern crate rustc_ast;

use cargo_support::{
    Manifest, dependencies, dependency_tables, emit_with_help, has_workspace_entry,
    package_manifest,
};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub WORKSPACE_DEPENDENCY_VERSIONS,
    Warn,
    "workspace package dependencies should inherit versions from workspace dependencies",
    WorkspaceDependencyVersions
}

impl EarlyLintPass for WorkspaceDependencyVersions {
    /// Check the crate's package manifest for dependencies with local versions.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(package) = package_manifest(cx, krate) {
            check_package(cx, &package);
        }
    }
}

/// Report every dependency of a workspace package that sets its own version.
///
/// Cargo rejects dependency tables in a virtual manifest, so every manifest
/// with dependencies here is a package manifest.
fn check_package(cx: &EarlyContext<'_>, package: &Manifest) {
    // Without `[workspace.dependencies]` there is nothing to inherit from.
    if !has_workspace_entry(cx, package, "dependencies") {
        return;
    }

    // Path-only and git-only dependencies have no version to move.
    for table in dependency_tables(package.root()) {
        for dependency in dependencies(table) {
            let Some((_, range)) = dependency.version() else {
                continue;
            };

            let name = dependency.name;
            emit_with_help(
                cx,
                WORKSPACE_DEPENDENCY_VERSIONS,
                package.span(range),
                &format!(
                    "dependency `{name}` specifies an explicit version instead of inheriting from `[workspace.dependencies]`"
                ),
                &format!(
                    "move the version for `{name}` into `[workspace.dependencies]` and use `{name}.workspace = true` here"
                ),
            );
        }
    }
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
