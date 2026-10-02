#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for bare rust-toolchain files.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::path::{Path, PathBuf};

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub RUST_TOOLCHAIN_TOML,
    Warn,
    "`rust-toolchain` file should use the `.toml` extension",
    RustToolchainToml
}

impl EarlyLintPass for RustToolchainToml {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Start from the real crate root source file so fixtures and workspace roots are handled
        // the same way rustc saw them during this compilation.
        let Some(crate_root) = crate_root_path(cx, krate.spans.inner_span) else {
            return;
        };

        if find_bare_toolchain(&crate_root).is_none() {
            return;
        }

        emit_span_lint_with_help(
            cx,
            RUST_TOOLCHAIN_TOML,
            crate_root_span,
            "repository uses `rust-toolchain` instead of `rust-toolchain.toml`",
            "rename `rust-toolchain` to `rust-toolchain.toml`",
        );
    }
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

/// Find bare toolchain used by the lint.
fn find_bare_toolchain(crate_root: &Path) -> Option<PathBuf> {
    // Walk upward because a toolchain file applies to every descendant crate.
    let mut directory = crate_root.parent();

    while let Some(path) = directory {
        // A bare rust-toolchain file applies to descendants, so any source ancestor can trigger.
        let candidate = path.join("rust-toolchain");
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
