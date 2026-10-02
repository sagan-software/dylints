#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]
#![warn(unused_extern_crates)]

//! A lint to check for machine-specific filesystem paths in string literals.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{Expr, ExprKind, token::LitKind};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub HARDCODED_PATH,
    Warn,
    "hardcoded machine-specific filesystem path",
    HardcodedPath
}

impl EarlyLintPass for HardcodedPath {
    /// Check one expression for a machine-specific string literal.
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        // Restrict the policy to source-written string literals before inspecting their contents.
        let ExprKind::Lit(literal) = &expr.kind else {
            return;
        };

        // Accept ordinary and raw strings because both can carry platform-specific paths.
        if !matches!(literal.kind, LitKind::Str | LitKind::StrRaw(_)) {
            return;
        }

        let Some(path_kind) = hardcoded_path_kind(literal.symbol.as_str()) else {
            return;
        };

        emit_hardcoded_path_lint(cx, expr.span, path_kind);
    }
}

/// The machine-specific path forms recognized by this lint.
#[derive(Copy, Clone)]
enum HardcodedPathKind {
    /// A path beginning at a filesystem root or Windows share.
    Absolute,
    /// A path whose first component depends on a user's home directory.
    HomeRelative,
}

impl HardcodedPathKind {
    /// Return the diagnostic message for this path form.
    const fn message(self) -> &'static str {
        match self {
            Self::Absolute => "hardcoded absolute filesystem path",
            Self::HomeRelative => "hardcoded home-relative filesystem path",
        }
    }
}

/// Classify a string literal when its lexical form identifies a machine-specific path.
fn hardcoded_path_kind(value: &str) -> Option<HardcodedPathKind> {
    // Check Windows drive and UNC forms before URI filtering because `C:/...` has a colon.
    if windows_absolute_path(value) || unix_absolute_path(value) {
        return (!is_web_url(value)).then_some(HardcodedPathKind::Absolute);
    }

    // A tilde path resolves through the process user's home directory and is not repo-relative.
    home_relative_path(value).then_some(HardcodedPathKind::HomeRelative)
}

/// Return whether a string is a Unix absolute path rather than a slash marker.
fn unix_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    let is_doc_marker =
        bytes.len() == 3 && bytes.starts_with(b"//") && matches!(bytes.get(2), Some(b'!' | b'/'));
    !is_doc_marker && value.starts_with('/') && value.bytes().any(|byte| byte != b'/')
}

/// Return whether a value is an absolute Windows drive or UNC path.
fn windows_absolute_path(value: &str) -> bool {
    let mut bytes = value.bytes();
    let drive_path = matches!(
        (bytes.next(), bytes.next(), bytes.next()),
        (Some(drive), Some(b':'), Some(separator))
            if drive.is_ascii_alphabetic() && matches!(separator, b'/' | b'\\')
    );
    let unc_path = value.as_bytes().starts_with(br"\\");

    drive_path || unc_path
}

/// Return whether a value uses shell-style home-directory expansion.
fn home_relative_path(value: &str) -> bool {
    // Match the shell forms that expand directly to the current user's home directory.
    let bytes = value.as_bytes();
    if matches!(bytes, [b'~']) || bytes.starts_with(b"~/") || bytes.starts_with(b"~\\") {
        return true;
    }

    // Also accept a named home expansion such as `~sagan/projects`.
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    *first == b'~' && rest.iter().any(|byte| matches!(*byte, b'/' | b'\\'))
}

/// Return whether a string is a web URL rather than a local absolute path.
fn is_web_url(value: &str) -> bool {
    ["http://", "https://", "ftp://", "ws://", "wss://"]
        .iter()
        .any(|scheme| value.starts_with(scheme))
}

/// Emit the diagnostic at the complete literal span with a portable-path suggestion.
fn emit_hardcoded_path_lint(cx: &EarlyContext<'_>, span: Span, path_kind: HardcodedPathKind) {
    cx.emit_span_lint(
        HARDCODED_PATH,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(path_kind.message());
            let _ = diag.help(
                "use a repository-relative path or derive the path from configuration or the runtime environment",
            );
        }),
    );
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
