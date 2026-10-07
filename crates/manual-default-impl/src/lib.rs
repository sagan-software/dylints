#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for manual Default implementations that could be derived.
//!
//! It resolves `Default` implementations for local, non-generic structs whose
//! `default` body is one struct expression with every field set to that field
//! type's default value. The resolved HIR and type-checking results decide each
//! case, so spelling, aliases, and comments do not matter. The fix replaces the
//! implementation with `#[derive(Default)]`, which produces the same value.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Expr, ExprKind, Impl, ImplItemKind, Item, ItemKind, LangItem, QPath, StructTailExpr,
    def::{CtorKind, DefKind, Res},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty, TypeckResults};
use rustc_span::{Span, def_id::DefId, kw, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_DEFAULT_IMPL,
    Warn,
    "manual `Default` implementation could be derived",
    ManualDefaultImpl
}

impl<'tcx> LateLintPass<'tcx> for ManualDefaultImpl {
    /// Check one `Default` implementation.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Filter unsupported implementations before resolving the default body.
        let Some((struct_id, self_ty)) = supported_default_struct(cx, item) else {
            return;
        };
        // Require one source-level `default` body before inspecting its expression.
        let ItemKind::Impl(Impl { items, .. }) = item.kind else {
            return;
        };

        // The single `default` body must build the struct from per-field defaults.
        let Some(body_id) = items.iter().find_map(|impl_item_id| {
            let ImplItemKind::Fn(_, body_id) = cx.tcx.hir_impl_item(*impl_item_id).kind else {
                return None;
            };
            Some(body_id)
        }) else {
            return;
        };
        let body = cx.tcx.hir_body(body_id);
        let typeck_results = cx.tcx.typeck_body(body_id);
        let ExprKind::Block(block, None) = body.value.kind else {
            return;
        };
        let is_default_body = is_default_body(cx, typeck_results, block, self_ty);
        if is_default_body {
            emit(cx, item, struct_id);
        }
    }
}

/// Return whether a body contains only default-valued fields.
fn is_default_body<'tcx>(
    cx: &LateContext<'tcx>,
    typeck_results: &TypeckResults<'tcx>,
    block: &rustc_hir::Block<'tcx>,
    self_ty: Ty<'tcx>,
) -> bool {
    let Some(expression) = block.expr else {
        return false;
    };
    block.stmts.is_empty() && is_built_from_defaults(cx, typeck_results, expression, self_ty)
}

/// Return a local struct and its resolved type for a concrete `Default` implementation.
fn supported_default_struct<'tcx>(
    cx: &LateContext<'tcx>,
    item: &Item<'tcx>,
) -> Option<(DefId, Ty<'tcx>)> {
    let ItemKind::Impl(Impl {
        of_trait: Some(_), ..
    }) = item.kind
    else {
        return None;
    };
    // Resolve the implemented trait and receiver before checking derive safety.
    let trait_ref = cx.tcx.impl_trait_ref(item.owner_id).skip_binder();
    let self_ty = trait_ref.self_ty();
    let ty::Adt(adt, args) = self_ty.kind() else {
        return None;
    };
    // Exclude generated implementations and types that cannot receive a derive fix.
    let is_source_authored = !item.span.from_expansion();
    let default_trait = cx.tcx.is_diagnostic_item(sym::Default, trait_ref.def_id);
    let is_local_struct = adt.is_struct() && adt.did().is_local();
    let is_concrete = args.non_erasable_generics().next().is_none();
    // The derive replacement is valid only after all these restrictions hold.
    (is_source_authored && default_trait && is_local_struct && is_concrete)
        .then_some((adt.did(), self_ty))
}

/// Return whether an expression builds the struct with only default field values.
fn is_built_from_defaults<'tcx>(
    cx: &LateContext<'tcx>,
    typeck_results: &TypeckResults<'tcx>,
    value: &Expr<'tcx>,
    self_ty: Ty<'tcx>,
) -> bool {
    let source_value = !value.span.from_expansion();
    let expected_type = typeck_results.expr_ty(value) == self_ty;
    if !source_value || !expected_type {
        return false;
    }
    // `Self { a: Default::default(), .. }` without a base expression.
    if let ExprKind::Struct(_, fields, StructTailExpr::None) = value.kind {
        return fields
            .iter()
            .all(|field| is_default_value(cx, typeck_results, field.expr));
    }

    // `Self(Default::default(), ..)` for a tuple struct.
    if let ExprKind::Call(callee, arguments) = value.kind {
        return is_struct_constructor(typeck_results, callee, CtorKind::Fn)
            && arguments
                .iter()
                .all(|argument| is_default_value(cx, typeck_results, argument));
    }

    // `Self` for a unit struct.
    if let ExprKind::Path(_) = value.kind {
        return is_struct_constructor(typeck_results, value, CtorKind::Const);
    }

    false
}

/// Return whether a path expression names the struct's constructor of the given kind.
fn is_struct_constructor(
    typeck_results: &TypeckResults<'_>,
    path: &Expr<'_>,
    kind: CtorKind,
) -> bool {
    // Constructors are resolved through the type-checked path rather than its spelling.
    let ExprKind::Path(ref qpath) = path.kind else {
        return false;
    };
    let resolution = typeck_results.qpath_res(qpath, path.hir_id);
    // Tuple and unit constructors carry their shape in the resolved definition kind.
    if let Res::Def(DefKind::Ctor(_, ctor_kind), _) = resolution {
        return ctor_kind == kind;
    }

    matches!(resolution, Res::SelfCtor(_))
}

/// Return whether one field value equals what `Default::default()` returns
/// for its type.
///
/// Accepted values are calls that resolve to `Default::default`, `false`, the
/// integer `0`, and `None`.
fn is_default_value<'tcx>(
    cx: &LateContext<'tcx>,
    typeck_results: &TypeckResults<'tcx>,
    value: &Expr<'tcx>,
) -> bool {
    if value.span.from_expansion() {
        return false;
    }
    // A default call must resolve to the standard trait method.
    let is_default_call = is_default_call_value(cx, typeck_results, value);
    // Literal defaults are limited to the values represented by derives.
    let is_default_literal = is_default_literal_value(value);
    // `None` is recognized through its resolved language item.
    let is_none = is_none_value_expression(cx, value);
    is_default_call || is_default_literal || is_none
}

/// Return whether an expression is a zero-argument default call.
fn is_default_call_value(
    cx: &LateContext<'_>,
    typeck_results: &TypeckResults<'_>,
    value: &Expr<'_>,
) -> bool {
    let ExprKind::Call(callee, []) = value.kind else {
        return false;
    };
    is_default_call(cx, typeck_results, callee)
}

/// Return whether an expression is a built-in literal default.
fn is_default_literal_value(value: &Expr<'_>) -> bool {
    let ExprKind::Lit(literal) = value.kind else {
        return false;
    };
    is_default_literal(literal.node)
}

/// Return whether an expression resolves to `Option::None`.
fn is_none_value_expression(cx: &LateContext<'_>, value: &Expr<'_>) -> bool {
    let ExprKind::Path(ref qpath @ QPath::Resolved(..)) = value.kind else {
        return false;
    };
    is_none_value(cx, qpath, value.hir_id)
}

/// Return whether a call resolves to `Default::default`.
fn is_default_call(
    cx: &LateContext<'_>,
    typeck_results: &TypeckResults<'_>,
    callee: &Expr<'_>,
) -> bool {
    let ExprKind::Path(ref qpath) = callee.kind else {
        return false;
    };
    typeck_results
        .qpath_res(qpath, callee.hir_id)
        .opt_def_id()
        .is_some_and(|def_id| is_default_fn(cx, def_id))
}

/// Return whether a literal is one of the built-in default values.
fn is_default_literal(literal: LitKind) -> bool {
    // Boolean false is the built-in default for `bool`.
    if let LitKind::Bool(value) = literal {
        return !value;
    }
    if let LitKind::Int(value, _) = literal {
        return value.get() == 0;
    }
    false
}

/// Return whether a resolved path names the language item's `Option::None`.
fn is_none_value(cx: &LateContext<'_>, qpath: &QPath<'_>, hir_id: rustc_hir::HirId) -> bool {
    cx.qpath_res(qpath, hir_id)
        .opt_def_id()
        .is_some_and(|def_id| {
            cx.tcx.lang_items().get(LangItem::OptionNone) == Some(cx.tcx.parent(def_id))
        })
}

/// Return whether a function is `Default::default` or an implementation of it.
fn is_default_fn(cx: &LateContext<'_>, def_id: DefId) -> bool {
    let tcx = cx.tcx;
    if tcx.is_diagnostic_item(sym::default_fn, def_id) {
        return true;
    }
    // An implementation's `default` resolves to the impl item; check its trait instead.
    let parent = tcx.parent(def_id);
    tcx.item_name(def_id) == kw::Default
        && matches!(tcx.def_kind(parent), DefKind::Impl { of_trait: true })
        && tcx.is_diagnostic_item(
            sym::Default,
            tcx.impl_trait_ref(parent).skip_binder().def_id,
        )
}

/// Emit the lint with a derive fix when the implementation can be removed cleanly.
fn emit(cx: &LateContext<'_>, item: &Item<'_>, struct_id: DefId) {
    let fix = derive_fix(cx, item, struct_id);
    cx.emit_span_lint(
        MANUAL_DEFAULT_IMPL,
        item.span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message("manual `Default` implementation looks derivable");
            let help = "derive `Default` instead; every field uses its type's default value";
            if let Some(fix) = fix {
                let _ = diag.multipart_suggestion(help, fix, Applicability::MachineApplicable);
            } else {
                let _ = diag.help(help);
            }
        }),
    );
}

/// Return the edits that add `#[derive(Default)]` to the struct and remove the impl.
///
/// The fix is offered only when neither item comes from a macro and the impl has
/// no attributes or doc comments that would be left without an item.
fn derive_fix(
    cx: &LateContext<'_>,
    item: &Item<'_>,
    struct_id: DefId,
) -> Option<Vec<(Span, String)>> {
    let struct_item = cx.tcx.hir_expect_item(struct_id.as_local()?);
    // Preserve attributes and source ownership when removing the implementation.
    let is_generated = struct_item.span.from_expansion();
    let has_attributes = !cx.tcx.hir_attrs(item.hir_id()).is_empty();
    if is_generated || has_attributes {
        return None;
    }
    // Derive placement follows the source indentation of the struct item.
    let source_map = cx.sess().source_map();
    let location = source_map.lookup_char_pos(struct_item.span.lo());
    let indentation: String = location
        .file
        .get_line(location.line.saturating_sub(1))?
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect();
    Some(vec![
        (
            struct_item.span.shrink_to_lo(),
            format!("#[derive(Default)]\n{indentation}"),
        ),
        (item.span, String::new()),
    ])
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::is_default_literal;
    use rustc_ast::{LitIntType, LitKind};

    /// Built-in literal defaults accept false and zero only.
    #[test]
    fn recognizes_builtin_literal_defaults() {
        assert!(
            is_default_literal(LitKind::Bool(false))
                && !is_default_literal(LitKind::Bool(true))
                && is_default_literal(LitKind::Int(0.into(), LitIntType::Unsuffixed))
                && !is_default_literal(LitKind::Int(1.into(), LitIntType::Unsuffixed))
        );
    }
}
