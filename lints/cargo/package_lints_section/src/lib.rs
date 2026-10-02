#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and slices validated ASCII Cargo table syntax"
)]

//! A lint to check for package manifests without a lints section.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub PACKAGE_LINTS_SECTION,
    Warn,
    "package manifests should define a `[lints]` table",
    PackageLintsSection
}

impl EarlyLintPass for PackageLintsSection {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Start at the crate root source file so path-based fixtures and real packages use the
        // same manifest lookup rule.
        let Some(crate_root) = crate_root_path(cx, crate_root_span) else {
            return;
        };

        let Some(manifest_path) = nearest_manifest(&crate_root) else {
            return;
        };

        // Unreadable manifests are skipped so generated or permission-limited source trees do not
        // get guessed diagnostics.
        let Some(manifest) = read_manifest(&manifest_path) else {
            return;
        };

        let tables = manifest_tables(&manifest);
        if !tables.has_package || tables.has_lints {
            return;
        }

        emit_span_lint_with_help(
            cx,
            PACKAGE_LINTS_SECTION,
            crate_root_span,
            "package `Cargo.toml` defines `[package]` but no `[lints]` table",
            "add a package `[lints]` table, usually with `workspace = true`",
        );
    }
}

/// Read manifest for source-based analysis.
fn read_manifest(path: &Path) -> Option<String> {
    let mut manifest = String::new();

    // Lint passes run synchronously inside rustc, so keep this dependency-free and local.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut manifest).ok()?;

    Some(manifest)
}

/// State used by the manifest tables analysis.
#[derive(Default)]
struct ManifestTables {
    /// has package stored for this lint's analysis.
    has_package: bool,
    /// has lints stored for this lint's analysis.
    has_lints: bool,
}

/// Helper for manifest tables analysis.
fn manifest_tables(manifest: &str) -> ManifestTables {
    // Track package presence separately from any package lint table.
    let mut tables = ManifestTables::default();

    for line in manifest.lines() {
        // Only table headers matter for this lint; key/value parsing is intentionally unnecessary.
        let Some(table) = table_header(line) else {
            continue;
        };

        if table == "package" {
            tables.has_package = true;
        } else if is_package_lints_table(table) {
            tables.has_lints = true;
        }
    }

    tables
}

/// Helper for table header analysis.
fn table_header(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with('#') || line.starts_with("[[") || !line.starts_with('[') {
        return None;
    }

    // Inline comments after the closing bracket are allowed, but malformed or empty table names do
    // not provide useful structure for this lint.
    let close = line.find(']')?;
    let table = line[1..close].trim();
    (!table.is_empty()).then_some(table)
}

/// Return whether package lints table.
fn is_package_lints_table(table: &str) -> bool {
    table == "lints" || table.starts_with("lints.")
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for crate root path analysis.
fn crate_root_path(cx: &EarlyContext<'_>, span: Span) -> Option<PathBuf> {
    // Virtual or path-remapped inputs may not have a readable local path, so skip those rather
    // than guessing from the process working directory.
    cx.sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
}

/// Helper for nearest manifest analysis.
fn nearest_manifest(crate_root: &Path) -> Option<PathBuf> {
    let mut directory = crate_root.parent();

    while let Some(path) = directory {
        // Cargo uses the nearest manifest for this crate root; stop at the first ancestor match.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }

        // Continue toward the filesystem root until a package boundary appears.
        directory = path.parent();
    }

    None
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
