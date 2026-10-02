#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the diagnostic builder methods return the builder for chaining"
)]

//! A lint to check for dependency versions without full semver.
//!
//! It parses the nearest `Cargo.toml` above the crate root with `toml_edit`,
//! visits every dependency table, including `target` and workspace tables,
//! and reports each one- or two-part numeric version requirement at its
//! string literal with a suggestion for the full `major.minor.patch` form.

extern crate rustc_ast;
extern crate rustc_errors;

use std::ops::Range;

use cargo_support::{
    Manifest, dependencies, dependency_tables, package_manifest, workspace_dependency_table,
};
use rustc_ast::Crate;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub DEPENDENCY_FULL_SEMVER_VERSIONS,
    Warn,
    "dependency version requirements should include a full semver patch component",
    DependencyFullSemverVersions
}

impl EarlyLintPass for DependencyFullSemverVersions {
    /// Check the crate's manifest for incomplete dependency versions.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(manifest) = package_manifest(cx, krate) {
            check_manifest(cx, &manifest);
        }
    }
}

/// Report every incomplete version in the manifest's dependency tables.
fn check_manifest(cx: &EarlyContext<'_>, manifest: &Manifest) {
    // Workspace dependencies declare the versions members inherit, so check them too.
    let root = manifest.root();
    for table in workspace_dependency_table(root)
        .into_iter()
        .chain(dependency_tables(root))
    {
        // Path-only and git-only dependencies have no version to check.
        for dependency in dependencies(table) {
            let Some((version, range)) = dependency.version() else {
                continue;
            };

            if let Some(full_version) = full_version(version) {
                emit_lint(cx, manifest, dependency.name, version, &full_version, range);
            }
        }
    }
}

/// Return the full `major.minor.patch` form of a one- or two-part numeric version.
///
/// Returns `None` for complete versions and for requirements with operators,
/// wildcards, or pre-release tags, which this lint does not check.
fn full_version(version: &str) -> Option<String> {
    // Operators, wildcards, and pre-release tags make a requirement this lint skips.
    if !is_numeric_requirement(version) {
        return None;
    }

    // Append only the missing components.
    let mut parts = version.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(_), None, _) => Some(format!("{version}.0.0")),
        (Some(_), Some(_), None) => Some(format!("{version}.0")),
        _ => None,
    }
}

/// Return whether every dot-separated part of `version` is a nonempty number.
fn is_numeric_requirement(version: &str) -> bool {
    version
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Emit the lint at the version literal with a suggested full version.
fn emit_lint(
    cx: &EarlyContext<'_>,
    manifest: &Manifest,
    name: &str,
    version: &str,
    full_version: &str,
    range: Range<usize>,
) {
    let span = manifest.span(range);
    let message = format!("dependency `{name}` has incomplete semver version `{version}`");
    let replacement = format!("\"{full_version}\"");

    cx.emit_span_lint(
        DEPENDENCY_FULL_SEMVER_VERSIONS,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            // Cargo reads both forms as the same requirement, but the edit lands in
            // `Cargo.toml`, which rustfix-based tools do not expect to rewrite.
            let _ = diag.span_suggestion(
                span,
                "write the full version",
                replacement,
                Applicability::MaybeIncorrect,
            );
        }),
    );
}

/// One- and two-part versions gain the missing zero components.
#[test]
fn completes_one_and_two_part_versions() {
    assert_eq!(full_version("1").as_deref(), Some("1.0.0"));
    assert_eq!(full_version("0.1").as_deref(), Some("0.1.0"));
}

/// Complete versions are not changed.
#[test]
fn skips_complete_versions() {
    assert_eq!(full_version("1.2.3"), None);
}

/// Versions with more than three components are not changed.
#[test]
fn skips_versions_with_extra_components() {
    assert_eq!(full_version("1.2.3.4"), None);
}

/// Operator requirements are not changed.
#[test]
fn skips_operator_requirements() {
    assert_eq!(full_version(">=1"), None);
}

/// Wildcard requirements are not changed.
#[test]
fn skips_wildcard_requirements() {
    assert_eq!(full_version("1.*"), None);
}

/// Empty numeric components are not changed.
#[test]
fn skips_trailing_empty_components() {
    assert_eq!(full_version("1."), None);
}

/// Leading empty components are not changed.
#[test]
fn skips_leading_empty_components() {
    assert_eq!(full_version(".1"), None);
}

/// Empty requirements are not changed.
#[test]
fn skips_empty_requirements() {
    assert_eq!(full_version(""), None);
}

/// Pre-release requirements are not changed.
#[test]
fn skips_pre_release_requirements() {
    assert_eq!(full_version("1.0-alpha"), None);
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
