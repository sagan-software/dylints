#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only tuple field types are relevant and rustc diagnostics are configured in place"
)]

//! A lint to check for path fields stored as strings.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::FieldDef;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, def_id::DefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub PATH_STRING_FIELD,
    Warn,
    "path-like field stored as a string",
    PathStringField
}

impl<'tcx> LateLintPass<'tcx> for PathStringField {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Resolve the field type only when its name denotes a filesystem path.
        let field_name = field.ident.name.to_ident_string();

        // Report raw strings at the field type span with a typed replacement.
        if path_field_name(&field_name)
            && let Some(string_ty) = string_ty(
                cx,
                cx.tcx
                    .type_of(field.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
            )
        {
            emit_span_lint_with_help(
                cx,
                PATH_STRING_FIELD,
                field.ty.span,
                format!("path-like field `{field_name}` uses `{string_ty}`"),
                "use `std::path::PathBuf`, `std::path::Path`, or a validated path newtype",
            );
        }
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

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

/// Return the path field name.
fn path_field_name(name: &str) -> bool {
    // Match common filesystem vocabulary while leaving arbitrary display strings alone.
    name.split('_').any(|token| {
        matches!(
            token,
            "path"
                | "paths"
                | "dir"
                | "dirs"
                | "directory"
                | "directories"
                | "folder"
                | "folders"
                | "file"
                | "filename"
                | "filepath"
        )
    })
}

/// Return type information for string.
fn string_ty(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(adt, _) if string_def_id(cx, adt.did()) => Some("String"),
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Str) => Some("&str"),
        _ => None,
    }
}

/// Helper for string def id analysis.
fn string_def_id(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(
        cx.tcx.def_path_str(def_id).as_str(),
        "alloc::string::String" | "std::string::String"
    )
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
