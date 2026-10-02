#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and slices validated ASCII Cargo table syntax"
)]

//! A lint to check for workspace packages that do not inherit workspace lints.
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
    pub WORKSPACE_LINTS_INHERITANCE,
    Warn,
    "workspace package lints should inherit from workspace lint settings",
    WorkspaceLintsInheritance
}

impl EarlyLintPass for WorkspaceLintsInheritance {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Start at the crate root source file so UI fixtures and normal packages use the same
        // nearest-manifest lookup rule.
        let Some(crate_root) = crate_root_path(cx, crate_root_span) else {
            return;
        };

        let Some(manifest_path) = nearest_manifest(&crate_root) else {
            return;
        };

        // Unreadable manifests can happen with generated or virtual source trees; skip them rather
        // than emitting a guessed repository-structure diagnostic.
        let Some(manifest) = read_manifest(&manifest_path) else {
            return;
        };

        let manifest_info = manifest_info(&manifest);
        if !should_warn(&manifest_path, &manifest_info) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            WORKSPACE_LINTS_INHERITANCE,
            crate_root_span,
            "workspace package defines `[lints]` without `workspace = true`",
            "add `workspace = true` to the package `[lints]` table",
        );
    }
}

/// Helper for should warn analysis.
fn should_warn(manifest_path: &Path, info: &ManifestInfo) -> bool {
    if !info.has_package || info.lints != LintsTable::Present {
        return false;
    }

    info.participates_in_workspace || ancestor_manifest_has_workspace(manifest_path)
}

/// Read manifest for source-based analysis.
fn read_manifest(path: &Path) -> Option<String> {
    let mut manifest = String::new();

    // Lint passes run inside rustc, so keep the manifest read local and dependency-free.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut manifest).ok()?;

    Some(manifest)
}

/// State used by the manifest info analysis.
#[derive(Default)]
struct ManifestInfo {
    /// has package stored for this lint's analysis.
    has_package: bool,
    /// lints stored for this lint's analysis.
    lints: LintsTable,
    /// participates in workspace stored for this lint's analysis.
    participates_in_workspace: bool,
}

/// Classification used by the lints table analysis.
#[derive(Default, PartialEq, Eq)]
enum LintsTable {
    #[default]
    /// Missing case used by this lint's analysis.
    Missing,
    /// present case used by this lint's analysis.
    Present,
    /// inherits workspace case used by this lint's analysis.
    InheritsWorkspace,
}

/// Helper for manifest info analysis.
fn manifest_info(manifest: &str) -> ManifestInfo {
    let mut info = ManifestInfo::default();
    let mut table = None;

    // Process table transitions and assignments in source order.
    for line in manifest.lines() {
        if let Some(header) = table_header(line) {
            // Only a few table names matter, but remember the active table for key parsing.
            info.has_package |= header == "package";
            if is_package_lints_table(header) && info.lints == LintsTable::Missing {
                info.lints = LintsTable::Present;
            }
            info.participates_in_workspace |= is_workspace_table(header);
            table = Some(header);
            continue;
        }

        let Some((key, value)) = key_value(line) else {
            continue;
        };

        // `workspace = true` is only the lint inheritance switch inside the package `[lints]`
        // table; other workspace keys are handled as package/workspace participation evidence.
        if table == Some("lints") && key == "workspace" && value_is_true(value) {
            info.lints = LintsTable::InheritsWorkspace;
        } else if workspace_inherited_key(key, value) {
            info.participates_in_workspace = true;
        }
    }

    info
}

/// Helper for ancestor manifest has workspace analysis.
fn ancestor_manifest_has_workspace(manifest_path: &Path) -> bool {
    let mut directory = manifest_path.parent().and_then(Path::parent);

    while let Some(path) = directory {
        let candidate = path.join("Cargo.toml");

        // Ancestor manifests are workspace context only when they explicitly define workspace
        // tables; unreadable ancestors are ignored rather than guessed.
        if candidate.is_file()
            && read_manifest(&candidate)
                .is_some_and(|manifest| manifest_has_workspace_table(&manifest))
        {
            return true;
        }

        directory = path.parent();
    }

    false
}

/// Helper for manifest has workspace table analysis.
fn manifest_has_workspace_table(manifest: &str) -> bool {
    manifest
        .lines()
        .filter_map(table_header)
        .any(is_workspace_table)
}

/// Helper for table header analysis.
fn table_header(line: &str) -> Option<&str> {
    // Ignore comments, arrays of tables, and non-table lines before slicing.
    let line = line.trim_start();
    if line.starts_with('#') || line.starts_with("[[") || !line.starts_with('[') {
        return None;
    }

    // Inline comments after the closing bracket are fine; malformed table headers are ignored.
    let close = line.find(']')?;
    let table = line[1..close].trim();
    (!table.is_empty()).then_some(table)
}

/// Return whether package lints table.
fn is_package_lints_table(table: &str) -> bool {
    table == "lints" || table.starts_with("lints.")
}

/// Return whether workspace table.
fn is_workspace_table(table: &str) -> bool {
    table == "workspace" || table.starts_with("workspace.")
}

/// Helper for key value analysis.
fn key_value(line: &str) -> Option<(&str, &str)> {
    // Parse only active key assignments and remove comments from their values.
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let (key, value) = line.split_once('=')?;
    Some((key.trim(), strip_inline_comment(value).trim()))
}

/// Helper for strip inline comment analysis.
fn strip_inline_comment(value: &str) -> &str {
    value.split_once('#').map_or(value, |(before, _)| before)
}

/// Helper for value is true analysis.
fn value_is_true(value: &str) -> bool {
    value.trim() == "true"
}

/// Helper for workspace inherited key analysis.
fn workspace_inherited_key(key: &str, value: &str) -> bool {
    key.ends_with(".workspace") && value_is_true(value)
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
    // Walk upward from the source file because nested crates may share a workspace.
    let mut directory = crate_root.parent();

    while let Some(path) = directory {
        // Cargo uses the nearest manifest for this crate root; stop at the first ancestor match.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }

        directory = path.parent();
    }

    None
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
