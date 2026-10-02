#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for value-based schemas of `JsonSchema` types.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use schemars as _;
#[cfg(test)]
use serde as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::def_id::LocalDefId;
use schemars_support::{is_schemars_method_call, local_json_schema_impls};

/// Stateful pass that caches local schema implementation targets.
///
/// The cache is populated once per crate and lets expression checks distinguish
/// local `JsonSchema` types from arbitrary values without repeating trait scans.
#[derive(Debug, Default)]
pub struct SchemarsSchemaForValueJsonSchema {
    /// Local ADTs with a semantically resolved `JsonSchema` implementation.
    implementations: Vec<LocalDefId>,
}

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub SCHEMARS_SCHEMA_FOR_VALUE_JSON_SCHEMA,
    Warn,
    "value-based schema generation is less precise for JsonSchema types",
    SchemarsSchemaForValueJsonSchema,
    SchemarsSchemaForValueJsonSchema::default()
}

impl<'tcx> LateLintPass<'tcx> for SchemarsSchemaForValueJsonSchema {
    /// Cache local `JsonSchema` implementations before checking calls.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.implementations = local_json_schema_impls(cx);
    }

    /// Check the two value-based generator methods used directly or by the macro.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the local schema target before checking the cached implementations.
        let Some(local_def_id) = local_schema_target(cx, expr) else {
            return;
        };
        if !self.implementations.contains(&local_def_id) {
            return;
        }

        // Report the source callsite so macro invocations receive a useful location.
        cx.emit_span_lint(
            SCHEMARS_SCHEMA_FOR_VALUE_JSON_SCHEMA,
            expr.span.source_callsite(),
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this value's type already implements `JsonSchema`")
                    .help("generate the schema from the type with `schema_for!(Type)`");
            }),
        );
    }
}

/// Resolve a supported value-based schema call to its local ADT target.
fn local_schema_target(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<LocalDefId> {
    // Require a supported one-value generator method before type analysis.
    let ExprKind::MethodCall(_, _, [value], _) = expr.kind else {
        return None;
    };
    if !matches!(
        schemars_method_name(cx, expr),
        Some("root_schema_for_value" | "into_root_schema_for_value")
    ) {
        return None;
    }

    // Normalize references and require a local ADT implementation target.
    let value_ty = cx.typeck_results().expr_ty(value).peel_refs();
    let ty::Adt(adt, _) = value_ty.kind() else {
        return None;
    };
    adt.did().as_local()
}

/// Resolve the supported Schemars method name without accepting user lookalikes.
fn schemars_method_name(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<&'static str> {
    ["root_schema_for_value", "into_root_schema_for_value"]
        .into_iter()
        .find(|name| is_schemars_method_call(cx, expr, name))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
