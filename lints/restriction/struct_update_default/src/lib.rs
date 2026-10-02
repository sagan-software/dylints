#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]
#![warn(unused_extern_crates)]

//! A lint to check for struct update syntax with a `Default::default()` base.
//!
//! It identifies a resolved `Default::default()` base hidden behind struct
//! update syntax and points at the source expression that conceals fields.
//! When the base comes from the built-in `#[derive(Default)]` of a local struct,
//! it suggests the explicit fields, because that derive defaults each field.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lexer;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Expr, ExprField, ExprKind, StructTailExpr, def_id::DefId};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Instance};
use rustc_span::{BytePos, Span, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub STRUCT_UPDATE_DEFAULT,
    Warn,
    "struct update syntax hides defaulted fields",
    StructUpdateDefault
}

impl<'tcx> LateLintPass<'tcx> for StructUpdateDefault {
    /// Check struct update bases for the resolved `Default::default` method.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Struct(_, fields, StructTailExpr::Base(base)) = expr.kind else {
            return;
        };
        let ExprKind::Call(callee, []) = base.kind else {
            return;
        };
        let Some(default_def_id) = called_def_id(cx, callee) else {
            return;
        };
        if !cx.tcx.is_diagnostic_item(sym::default_fn, default_def_id) {
            return;
        }

        // Suggest explicit fields only when the rewrite provably keeps the same values.
        let suggestion = explicit_fields_suggestion(cx, expr, fields, base, callee, default_def_id);

        // Point at the hidden default base that should become explicit fields.
        cx.emit_span_lint(
            STRUCT_UPDATE_DEFAULT,
            base.span,
            DiagDecorator(|diag| {
                let _ = diag.primary_message(
                    "struct update syntax hides fields behind `Default::default()`",
                );
                if let Some((span, replacement)) = suggestion {
                    let _ = diag.span_suggestion(
                        span,
                        "set the remaining fields explicitly",
                        replacement,
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _ = diag.help("set every field explicitly at the construction boundary");
                }
            }),
        );
    }
}

/// Build the `..Default::default()` replacement for remaining fields.
///
/// The rewrite is exact only for a local struct with built-in derived defaults.
fn explicit_fields_suggestion<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    fields: &[ExprField<'tcx>],
    base: &Expr<'tcx>,
    callee: &Expr<'tcx>,
    default_def_id: DefId,
) -> Option<(Span, String)> {
    if expr.span.from_expansion() || base.span.from_expansion() {
        return None;
    }

    // Require the base call to resolve to a built-in derived implementation.
    if !has_builtin_default_impl(cx, callee, default_def_id) {
        return None;
    }

    // Keep only remaining fields whose derive has no custom default value.
    let remaining = remaining_default_fields(cx, expr, fields)?;

    // Replace `..Default::default()` and keep the indentation of the `..` line.
    let span = dot_dot_span(cx, expr.span, base.span)?;
    let separator =
        line_indent(cx, span).map_or_else(|| ", ".to_owned(), |indent| format!(",\n{indent}"));
    let replacement = remaining
        .iter()
        .map(|name| format!("{name}: Default::default()"))
        .collect::<Vec<_>>()
        .join(&separator);
    Some((span, replacement))
}

/// Return whether the base call uses a compiler-recognized derived default implementation.
fn has_builtin_default_impl(
    cx: &LateContext<'_>,
    callee: &Expr<'_>,
    default_def_id: DefId,
) -> bool {
    // Resolve the call with its inferred generic arguments.
    let args = cx.typeck_results().node_args(callee.hir_id);
    // The compiler can identify the concrete implementation only after substitution.
    let Some(instance) = Instance::try_resolve(cx.tcx, cx.typing_env(), default_def_id, args)
        .ok()
        .flatten()
    else {
        return false;
    };
    // A resolved associated function belongs to the derived implementation's owner.
    let Some(impl_def_id) = cx.tcx.impl_of_assoc(instance.def_id()) else {
        return false;
    };
    cx.tcx.is_builtin_derived(impl_def_id)
}

/// Return the names of fields that the derived default would initialize.
fn remaining_default_fields<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    fields: &[ExprField<'tcx>],
) -> Option<Vec<rustc_span::Symbol>> {
    // Resolve the constructed value and require a struct ADT.
    let ty::Adt(adt, _) = cx.typeck_results().expr_ty(expr).kind() else {
        return None;
    };
    if !adt.is_struct() {
        return None;
    }
    // Preserve declaration order while excluding fields already written by the caller.
    let remaining = adt
        .non_enum_variant()
        .fields
        .iter()
        .filter(|field| {
            !fields
                .iter()
                .any(|written| written.ident.name == field.name)
        })
        .map(|field| field.name)
        .collect::<Vec<_>>();
    // Custom field defaults would make the explicit rewrite observably different.
    (!adt
        .non_enum_variant()
        .fields
        .iter()
        .filter(|field| {
            !fields
                .iter()
                .any(|written| written.ident.name == field.name)
        })
        .any(|field| field.value.is_some()))
    .then_some(remaining)
}

/// Return the definition that a call's callee resolves to.
fn called_def_id(cx: &LateContext<'_>, callee: &Expr<'_>) -> Option<DefId> {
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };
    cx.qpath_res(&qpath, callee.hir_id).opt_def_id()
}

/// Return the span from the `..` token through the end of the base expression.
fn dot_dot_span(cx: &LateContext<'_>, struct_span: Span, base_span: Span) -> Option<Span> {
    // Read the source before the base expression so token offsets stay source-relative.
    let before_base = cx
        .sess()
        .source_map()
        .span_to_snippet(struct_span.with_hi(base_span.lo()))
        .ok()?;

    // Lex the text so `..` inside a comment cannot be mistaken for the update token.
    let mut offset = 0_u32;
    let mut last_tokens = [None, None];
    // Keep only non-whitespace tokens because spacing does not identify the update operator.
    for token in tokenize(&before_base, FrontmatterAllowed::No) {
        if !matches!(token.kind, TokenKind::Whitespace) {
            last_tokens = [last_tokens[1], Some((token.kind, offset))];
        }
        offset = offset.checked_add(token.len)?;
    }

    // Require `..` as the last two tokens, so no comment sits between it and the base.
    let [
        Some((TokenKind::Dot, start)),
        Some((TokenKind::Dot, second)),
    ] = last_tokens
    else {
        return None;
    };
    (second == start + 1).then(|| base_span.with_lo(struct_span.lo() + BytePos(start)))
}

/// Return the whitespace before `span` when it starts its own source line.
fn line_indent(cx: &LateContext<'_>, span: Span) -> Option<String> {
    let source_map = cx.sess().source_map();
    let line_start = source_map.span_extend_to_prev_char(span.shrink_to_lo(), '\n', true);
    let indent = source_map.span_to_snippet(line_start).ok()?;
    indent
        .chars()
        .all(|character| character == ' ' || character == '\t')
        .then_some(indent)
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
