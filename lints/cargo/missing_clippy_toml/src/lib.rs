#![feature(rustc_private)]
#![expect(
    clippy::string_slice,
    clippy::let_underscore_must_use,
    reason = "the parser slices ASCII TOML delimiters and configures rustc diagnostics in place"
)]

//! A lint to check for missing clippy.toml files.
//!
//! It locates the Cargo package root for the current crate, checks whether the
//! package defines a Clippy configuration, and reports the missing file at a
//! stable source span. The rule keeps workspace and dependency paths separate
//! so each package can state its own lint policy without guessing ownership.

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
    pub MISSING_CLIPPY_TOML,
    Warn,
    "crates should have a clippy.toml file",
    MissingClippyToml
}

impl EarlyLintPass for MissingClippyToml {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Anchor the search to the source file rustc compiled, not the process working directory.
        let Some(crate_root) = crate_root_path(cx, crate_root_span) else {
            return;
        };

        // Derive the package and workspace bounds before searching for configuration.
        let Some(search) = clippy_config_search(&crate_root) else {
            return;
        };

        if has_clippy_toml(&search.crate_dir, &search.root_dir) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            MISSING_CLIPPY_TOML,
            crate_root_span,
            "crate has no `clippy.toml` file",
            "add `clippy.toml` in the crate directory or workspace root",
        );
    }
}

/// State used by the clippy config search analysis.
struct ClippyConfigSearch {
    /// crate dir stored for this lint's analysis.
    crate_dir: PathBuf,
    /// root dir stored for this lint's analysis.
    root_dir: PathBuf,
}

/// Helper for clippy config search analysis.
fn clippy_config_search(crate_root: &Path) -> Option<ClippyConfigSearch> {
    // Prefer Cargo metadata discovered from the compiled source path.
    if let Some(manifest_path) = nearest_manifest(crate_root) {
        let crate_dir = manifest_path.parent()?.to_path_buf();
        let root_dir = workspace_root(&manifest_path).unwrap_or_else(|| crate_dir.clone());

        return Some(ClippyConfigSearch {
            crate_dir,
            root_dir,
        });
    }

    // Non-Cargo rustc inputs still get a local project check, but only for the source directory.
    let crate_dir = crate_root.parent()?.to_path_buf();
    Some(ClippyConfigSearch {
        crate_dir: crate_dir.clone(),
        root_dir: crate_dir,
    })
}

/// Helper for nearest manifest analysis.
fn nearest_manifest(crate_root: &Path) -> Option<PathBuf> {
    // Walk upward from the source file because nested crates may share a workspace.
    let mut directory = crate_root.parent();

    while let Some(path) = directory {
        // The nearest manifest is the compiled crate's package/project boundary.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }

        directory = path.parent();
    }

    // A source outside a Cargo package has no manifest boundary.
    None
}

/// Helper for workspace root analysis.
fn workspace_root(manifest_path: &Path) -> Option<PathBuf> {
    let manifest_dir = manifest_path.parent()?;
    // A manifest with its own workspace table defines the search root directly.
    if manifest_has_workspace(manifest_path) {
        return Some(manifest_dir.to_path_buf());
    }

    let mut directory = manifest_dir.parent();
    while let Some(path) = directory {
        // Stop at the first ancestor manifest with workspace tables; farther ancestors are outside
        // this Cargo workspace.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() && manifest_has_workspace(&candidate) {
            return Some(path.to_path_buf());
        }

        directory = path.parent();
    }

    None
}

/// Helper for manifest has workspace analysis.
fn manifest_has_workspace(path: &Path) -> bool {
    // Unreadable manifests provide no reliable project boundary, so ignore them.
    read_manifest(path).is_some_and(|manifest| {
        manifest
            .lines()
            .filter_map(table_header)
            .any(is_workspace_table)
    })
}

/// Read manifest for source-based analysis.
fn read_manifest(path: &Path) -> Option<String> {
    let mut manifest = String::new();

    // Lint passes run synchronously inside rustc, so keep this local and dependency-free.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut manifest).ok()?;

    Some(manifest)
}

/// Helper for table header analysis.
fn table_header(line: &str) -> Option<&str> {
    // Ignore comments, arrays of tables, and non-table lines before slicing.
    let line = line.trim_start();
    if line.starts_with('#') || line.starts_with("[[") || !line.starts_with('[') {
        return None;
    }

    // Inline comments after the closing bracket are allowed; malformed table names are ignored.
    let close = line.find(']')?;
    let table = line[1..close].trim();
    (!table.is_empty()).then_some(table)
}

/// Return whether workspace table.
fn is_workspace_table(table: &str) -> bool {
    table == "workspace" || table.starts_with("workspace.")
}

/// Return whether clippy toml is present.
fn has_clippy_toml(crate_dir: &Path, root_dir: &Path) -> bool {
    // Search from the package toward the resolved workspace boundary.
    let mut directory = Some(crate_dir);

    while let Some(path) = directory {
        // Any clippy.toml in the crate-to-root path configures this project.
        if path.join("clippy.toml").is_file() {
            return true;
        }

        if path == root_dir {
            // Do not allow an unrelated ancestor configuration to satisfy this workspace.
            break;
        }

        directory = path.parent();
    }

    false
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

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
