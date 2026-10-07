#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "rustc syntax enums need a forward-compatible fallback"
)]

//! A lint to check for `JsonSchema::json_schema` methods returning `$ref` schemas.
//!
//! This Dylint library resolves Schemars schema constructors, reports reference
//! schemas returned from value methods, and recommends the appropriate schema API.
//!
//! The README defines the supported return shapes and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use schemars as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Expr, ExprKind, ImplItem, ImplItemImplKind, ImplItemKind,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use schemars_support::{is_json_schema_trait, is_schemars_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SCHEMARS_JSON_SCHEMA_REF_RETURN,
    Warn,
    "JsonSchema::json_schema must not return a reference schema",
    SchemarsJsonSchemaRefReturn
}

impl<'tcx> LateLintPass<'tcx> for SchemarsJsonSchemaRefReturn {
    /// Check resolved `JsonSchema::json_schema` method result paths.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict traversal to resolved implementations of the exact trait method.
        let Some(body) = json_schema_body(cx, item) else {
            return;
        };
        // Check the tail result before walking explicit returns in the same body.
        let body = cx.tcx.hir_body(body);
        let span = direct_ref_result(cx, body.value).or_else(|| {
            let mut visitor = ReturnedRefVisitor { cx, span: None };
            visitor.visit_expr(body.value);
            visitor.span
        });
        let Some(span) = span else {
            return;
        };

        cx.emit_span_lint(
            SCHEMARS_JSON_SCHEMA_REF_RETURN,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("`JsonSchema::json_schema` returns a `$ref` schema")
                    .help("return a concrete schema and use `generator.subschema_for` for dependencies");
            }),
        );
    }
}

/// Return the body for a resolved Schemars `JsonSchema::json_schema` method.
fn json_schema_body(cx: &LateContext<'_>, item: &ImplItem<'_>) -> Option<rustc_hir::BodyId> {
    // Require a trait method with the exact public method name.
    let ImplItemKind::Fn(_, body) = item.kind else {
        return None;
    };
    if item.ident.name.as_str() != "json_schema"
        || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. })
    {
        return None;
    }

    // Resolve the implemented trait to exclude unrelated same-named methods.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;
    is_json_schema_trait(cx, trait_def_id).then_some(body)
}

/// Visitor that considers only direct function result positions.
struct ReturnedRefVisitor<'a, 'tcx> {
    /// Active late-lint context.
    cx: &'a LateContext<'tcx>,
    /// First direct reference-schema return.
    span: Option<Span>,
}

impl<'tcx> Visitor<'tcx> for ReturnedRefVisitor<'_, 'tcx> {
    /// Inspect the tail expression and explicit `return` expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Stop after the first reportable return has been located.
        if self.span.is_some() {
            return;
        }

        // Inspect only explicit returns while ordinary traversal handles nested expressions.
        if let ExprKind::Ret(Some(value)) = expr.kind
            && let Some(span) = direct_ref_result(self.cx, value)
        {
            self.span = Some(span);
            return;
        }

        walk_expr(self, expr);
    }

    /// Do not attribute reference schemas returned by nested closures.
    fn visit_nested_body(&mut self, _body: rustc_hir::BodyId) {}
}

/// Find `Schema::new_ref` in an expression used directly as a function result.
fn direct_ref_result(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Report a direct constructor call before peeling expression wrappers.
    is_schemars_function_call(cx, expr, "new_ref")
        .then_some(expr.span)
        .or_else(|| match expr.kind {
            // Follow block tails and compiler temporaries without entering nested bodies.
            ExprKind::Block(block, _) => block.expr.and_then(|tail| direct_ref_result(cx, tail)),
            ExprKind::DropTemps(inner) => direct_ref_result(cx, inner),
            // Search each result-producing control-flow branch in source order.
            ExprKind::If(_, then_branch, else_branch) => direct_ref_result(cx, then_branch)
                .or_else(|| else_branch.and_then(|branch| direct_ref_result(cx, branch))),
            ExprKind::Match(_, arms, _) => {
                arms.iter().find_map(|arm| direct_ref_result(cx, arm.body))
            }
            _ => None,
        })
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
