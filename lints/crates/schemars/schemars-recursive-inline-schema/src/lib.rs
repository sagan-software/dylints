#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for recursive Schemars schemas marked inline.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use schemars as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, ImplItem, ImplItemImplKind, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use schemars_support::is_json_schema_trait;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SCHEMARS_RECURSIVE_INLINE_SCHEMA,
    Warn,
    "recursive Schemars schema must not be inlined",
    SchemarsRecursiveInlineSchema
}

impl<'tcx> LateLintPass<'tcx> for SchemarsRecursiveInlineSchema {
    /// Check resolved `JsonSchema::inline_schema` implementations.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let Some((self_ty, body)) = inline_schema_body(cx, item) else {
            return;
        };
        let ty::Adt(adt, arguments) = self_ty.kind() else {
            return;
        };
        if !returns_true(cx.tcx.hir_body(body).value) {
            return;
        }

        // Resolve each field type so aliases and wrapper types still expose direct recursion.
        let is_recursive = adt.all_fields().any(|field| {
            contains_adt(
                cx.tcx
                    .type_of(field.did)
                    .instantiate(cx.tcx, arguments)
                    .skip_norm_wip(),
                adt.did(),
            )
        });
        if !is_recursive {
            return;
        }

        // Report the recursive type after both inline policy and recursion are proved.
        cx.emit_span_lint(
            SCHEMARS_RECURSIVE_INLINE_SCHEMA,
            cx.tcx.def_span(adt.did()),
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this recursive type is configured to inline its schema")
                    .help("remove `#[schemars(inline)]` or return `false` from `inline_schema`");
            }),
        );
    }
}

/// Return the implementing type and body for Schemars's `inline_schema` method.
fn inline_schema_body<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx ImplItem<'tcx>,
) -> Option<(Ty<'tcx>, rustc_hir::BodyId)> {
    // Require a trait method with the exact inline-policy method name.
    let ImplItemKind::Fn(_, body) = item.kind else {
        return None;
    };
    if item.ident.name.as_str() != "inline_schema"
        || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. })
    {
        return None;
    }

    // Resolve the implemented trait before returning its self type and body.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;
    is_json_schema_trait(cx, trait_def_id).then(|| {
        (
            cx.tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip(),
            body,
        )
    })
}

/// Return whether a method body is the constant value `true`.
fn returns_true(expr: &Expr<'_>) -> bool {
    // Match the direct boolean literal before peeling wrappers.
    if let ExprKind::Lit(literal) = expr.kind {
        return matches!(literal.node, rustc_ast::LitKind::Bool(true));
    }
    // Recurse through block tails and compiler-generated temporaries.
    if let ExprKind::Block(block, _) = expr.kind {
        return block.expr.is_some_and(returns_true);
    }
    if let ExprKind::DropTemps(inner) = expr.kind {
        return returns_true(inner);
    }
    false
}

/// Return whether a field type directly contains the implementing ADT.
fn contains_adt(ty: Ty<'_>, target: DefId) -> bool {
    // Inspect an ADT and its generic type arguments first.
    if let ty::Adt(adt, arguments) = ty.kind() {
        return adt.did() == target
            || arguments
                .types()
                .any(|argument| contains_adt(argument, target));
    }
    // Recurse through containers and tuple elements that can hold the target ADT.
    if let ty::Array(inner, _) | ty::Slice(inner) | ty::RawPtr(inner, _) | ty::Ref(_, inner, _) =
        ty.kind()
    {
        return contains_adt(*inner, target);
    }
    if let ty::Tuple(types) = ty.kind() {
        return types.iter().any(|inner| contains_adt(inner, target));
    }
    false
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
