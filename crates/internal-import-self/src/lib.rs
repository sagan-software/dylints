#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "the lint targets import items and configures rustc diagnostics in place"
)]

//! A lint to check for bare sibling-module imports that should use `self::`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    HirId, ItemKind, Mod, UsePath,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, def_id::DefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INTERNAL_IMPORT_SELF,
    Warn,
    "internal import should start with self::",
    InternalImportSelf
}

impl<'tcx> LateLintPass<'tcx> for InternalImportSelf {
    /// Check mod for this lint.
    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, _hir_id: HirId) {
        check_module_items(cx, module);
    }
}

/// Check module items for this lint.
fn check_module_items<'tcx>(cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>) {
    // Resolve direct child modules before classifying bare import roots.
    let child_modules = module
        .item_ids
        .iter()
        .filter_map(|item_id| match cx.tcx.hir_item(*item_id).kind {
            ItemKind::Mod(..) => Some(item_id.owner_id.to_def_id()),
            _ => None,
        })
        .collect::<Vec<_>>();

    // Inspect only use items because other item paths do not express imports.
    for item_id in module.item_ids {
        let item = cx.tcx.hir_item(*item_id);
        if let ItemKind::Use(path, _) = item.kind {
            check_use_path(cx, path, &child_modules);
        }
    }
}

/// Check use path for this lint.
fn check_use_path(cx: &LateContext<'_>, path: &UsePath<'_>, child_modules: &[DefId]) {
    let Some(first_segment) = path.segments.first().and_then(|segment| match segment.res {
        Res::Def(DefKind::Mod, def_id) => Some(def_id),
        _ => None,
    }) else {
        return;
    };

    // Require rustc to prove that the bare first segment is a direct child module of the module
    // containing this import, so external crates and explicit roots stay quiet.
    if !child_modules.contains(&first_segment) {
        return;
    }

    // Grouped imports lower to leaf spans such as `nested::Deep` in `use root::{nested::Deep}`,
    // so only suggest when the span starts at the first segment and holds the full path.
    let starts_at_root = path
        .segments
        .first()
        .is_some_and(|segment| segment.ident.span.lo() == path.span.lo());
    let suggestion = cx
        .sess()
        .source_map()
        .span_to_snippet(path.span)
        .ok()
        .filter(|snippet| starts_at_root && snippet.contains("::"))
        .map(|snippet| format!("self::{snippet}"));

    emit_span_lint_with_help(
        cx,
        INTERNAL_IMPORT_SELF,
        path.span,
        "internal import should start with `self::`",
        "add `self::` to this import path",
        suggestion,
    );
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: Option<String>,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(message);
            if let Some(suggestion) = suggestion {
                let _ =
                    diag.span_suggestion(span, help, suggestion, Applicability::MachineApplicable);
            } else {
                let _ = diag.help(help);
            }
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
