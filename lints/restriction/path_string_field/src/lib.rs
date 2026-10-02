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

use rustc_errors::DiagDecorator;
use rustc_hir::{FieldDef, LangItem};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub PATH_STRING_FIELD,
    Warn,
    "path-like field stored as a string",
    PathStringField
}

impl<'tcx> LateLintPass<'tcx> for PathStringField {
    /// Check one named field whose name denotes a filesystem path.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Resolve the field type only when its name denotes a filesystem path.
        let field_name = field.ident.name.to_ident_string();
        if !path_field_name(&field_name) {
            return;
        }
        let field_ty = cx
            .tcx
            .type_of(field.def_id)
            .instantiate_identity()
            .skip_norm_wip();
        let Some(string_ty) = string_ty(cx, field_ty) else {
            return;
        };

        // Report raw strings at the field type span with a typed replacement.
        cx.emit_span_lint(
            PATH_STRING_FIELD,
            field.ty.span,
            DiagDecorator(|diag| {
                let _ = diag
                    .primary_message(format!("path-like field `{field_name}` uses `{string_ty}`"));
                let _ = diag.help(
                    "use `std::path::PathBuf`, `std::path::Path`, or a validated path newtype",
                );
            }),
        );
    }
}

/// Return whether a field name denotes a filesystem path.
fn path_field_name(name: &str) -> bool {
    let mut tokens = name.split('_');

    // A qualifier such as `url_path`, `module_path`, or `public_path` names a non-file path.
    let has_filesystem_token = tokens.clone().any(|token| {
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
    });
    has_filesystem_token
        && !tokens.any(|token| {
            matches!(
                token,
                "api"
                    | "crate"
                    | "def"
                    | "endpoint"
                    | "http"
                    | "import"
                    | "item"
                    | "json"
                    | "key"
                    | "mod"
                    | "module"
                    | "public"
                    | "query"
                    | "route"
                    | "symbol"
                    | "type"
                    | "uri"
                    | "url"
                    | "web"
                    | "xpath"
            )
        })
}

/// Return the written form of a resolved `String` or `&str` type.
fn string_ty(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(adt, _) if cx.tcx.is_lang_item(adt.did(), LangItem::String) => Some("String"),
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Str) => Some("&str"),
        _ => None,
    }
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
