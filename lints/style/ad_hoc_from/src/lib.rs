#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]
#![warn(unused_extern_crates)]

//! A lint to check for ad hoc infallible conversion functions.
//!
//! It checks free functions and inherent associated functions whose name and
//! resolved one-argument signature describe a conversion that a `From`
//! implementation could express. It skips receiver methods, conversions whose
//! `From` implementation already exists or would break coherence, and bodies
//! that can panic on bad input, because `From` promises an infallible conversion.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprKind, ImplItem, ImplItemImplKind, ImplItemKind,
    def::DefKind,
    intravisit::{self, FnKind, Visitor},
};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeckResults};
use rustc_span::{Span, Symbol, def_id::LocalDefId, sym};
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_FROM,
    Warn,
    "ad hoc infallible conversion function could be `From`",
    AdHocFrom
}

/// Diagnostic names of the standard macros that panic.
const PANIC_MACROS: &[&str] = &[
    "assert_eq_macro",
    "assert_macro",
    "assert_ne_macro",
    "core_panic_2015_macro",
    "core_panic_2021_macro",
    "core_panic_macro",
    "std_panic_macro",
    "todo_macro",
    "unimplemented_macro",
    "unreachable_2015_macro",
    "unreachable_macro",
];

impl<'tcx> LateLintPass<'tcx> for AdHocFrom {
    /// Check one free function.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if let FnKind::ItemFn(ident, ..) = kind {
            check_candidate(cx, ident.name, body, span, local_def_id);
        }
    }

    /// Check one inherent associated function; trait items have names fixed by their trait.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Trait methods keep their trait-defined conversion contract and stay out of scope.
        let ImplItemKind::Fn(_, body_id) = item.kind else {
            return;
        };
        let def_id = item.owner_id.def_id;
        if matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            && !cx.tcx.associated_item(def_id).is_method()
        {
            check_candidate(
                cx,
                item.ident.name,
                cx.tcx.hir_body(body_id),
                item.span,
                def_id,
            );
        }
    }
}

/// Report one function whose name, signature, and body fit an infallible `From` conversion.
fn check_candidate<'tcx>(
    cx: &LateContext<'tcx>,
    name: Symbol,
    body: &'tcx Body<'tcx>,
    span: Span,
    local_def_id: LocalDefId,
) {
    // Resolve conversion vocabulary and the one-input signature before checking coherence.
    let Some((input, output)) = conversion_signature(cx, name, local_def_id) else {
        return;
    };

    // Require a legal, new `From` impl: a local side, distinct types, and no existing impl.
    if !is_valid_from_conversion(cx, input, output) {
        return;
    }

    // `From` must not fail, so a body that panics on bad input needs `TryFrom` instead.
    let mut panics = PanicFinder {
        tcx: cx.tcx,
        typeck_results: cx.tcx.typeck_body(body.id()),
        has_panic: false,
    };
    panics.visit_expr(body.value);
    if panics.has_panic {
        return;
    }

    cx.emit_span_lint(
        AD_HOC_FROM,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(format!(
                "function `{name}` looks like an infallible conversion"
            ));
            let _ = diag.help(format!(
                "implement `From<{input}> for {output}` when the conversion has one canonical meaning"
            ));
        }),
    );
}

/// Return the one-input signature of a named infallible conversion.
fn conversion_signature<'tcx>(
    cx: &LateContext<'tcx>,
    name: Symbol,
    local_def_id: LocalDefId,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    // Resolve the conversion signature before inspecting its trait implementation.
    if !conversion_name(name.as_str()) {
        return None;
    }
    // Erase late-bound lifetimes so the trait solver sees closed types.
    let fn_sig = cx.tcx.instantiate_bound_regions_with_erased(
        cx.tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    );
    let [input] = fn_sig.inputs() else {
        return None;
    };
    Some((*input, fn_sig.output()))
}

/// Return whether a conversion satisfies the `From` coherence and type rules.
fn is_valid_from_conversion<'tcx>(
    cx: &LateContext<'tcx>,
    input: Ty<'tcx>,
    output: Ty<'tcx>,
) -> bool {
    concrete_target_type(cx, output)
        && meaningful_input(input)
        && input != output
        && (outer_local_type(input) || outer_local_type(output))
        && !has_from_impl(cx, output, input)
}

/// Return whether the name uses conversion vocabulary without joining two operations.
fn conversion_name(name: &str) -> bool {
    [
        "make_",
        "build_",
        "convert_",
        "extract_",
        "extracted_",
        "from_",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
        && !name.contains("_and_")
}

/// Return whether the outer type, after borrowing, is defined in this crate.
fn outer_local_type(mut ty: Ty<'_>) -> bool {
    // Peel references because `&T` is fundamental for the orphan rule.
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
    }

    matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local())
}

/// Return whether the output is a struct, enum, or union other than `Option` or `Result`.
fn concrete_target_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    !cx.tcx.is_diagnostic_item(sym::Result, adt.did())
        && !cx.tcx.is_diagnostic_item(sym::Option, adt.did())
}

/// Return whether the input can be the source of a `From` impl.
///
/// A bare type parameter is rejected because `impl<T> From<T> for Target` overlaps
/// the standard reflexive `impl<T> From<T> for T`.
fn meaningful_input(ty: Ty<'_>) -> bool {
    !matches!(ty.kind(), ty::Never | ty::Param(_))
        && !matches!(ty.kind(), ty::Tuple(fields) if fields.is_empty())
}

/// Return whether `target: From<source>` may already hold in the item's environment.
fn has_from_impl<'tcx>(cx: &LateContext<'tcx>, target: Ty<'tcx>, source: Ty<'tcx>) -> bool {
    cx.tcx.get_diagnostic_item(sym::From).is_none_or(|from| {
        cx.tcx
            .infer_ctxt()
            .build(cx.typing_mode())
            .type_implements_trait(from, [target, source], cx.param_env)
            .may_apply()
    })
}

/// Finds a standard panic macro or an `unwrap`-style call in one body.
struct PanicFinder<'tcx> {
    /// Compiler context used to resolve macros and methods.
    tcx: TyCtxt<'tcx>,
    /// Type-checking results of the inspected body.
    typeck_results: &'tcx TypeckResults<'tcx>,
    /// Whether a panicking expression was found.
    has_panic: bool,
}

impl<'tcx> Visitor<'tcx> for PanicFinder<'tcx> {
    /// Stop at the first expression that can panic.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.has_panic {
            return;
        }
        self.has_panic = self.is_panic_macro(expr.span) || self.is_unwrap_call(expr);
        intravisit::walk_expr(self, expr);
    }
}

impl PanicFinder<'_> {
    /// Return whether the span was produced by a standard panicking macro.
    fn is_panic_macro(&self, span: Span) -> bool {
        span.macro_backtrace().any(|expansion| {
            expansion
                .macro_def_id
                .and_then(|macro_id| self.tcx.get_diagnostic_name(macro_id))
                .is_some_and(|name| PANIC_MACROS.contains(&name.as_str()))
        })
    }

    /// Return whether the expression calls `unwrap` or `expect` on an `Option` or `Result`.
    fn is_unwrap_call(&self, expr: &Expr<'_>) -> bool {
        let ExprKind::MethodCall(..) = expr.kind else {
            return false;
        };
        let tcx = self.tcx;
        // Resolve the called method and check both its name and its owning type.
        self.typeck_results
            .type_dependent_def_id(expr.hir_id)
            .is_some_and(|method| {
                let owner = tcx.parent(method);
                matches!(
                    tcx.item_name(method).as_str(),
                    "unwrap" | "expect" | "unwrap_err" | "expect_err"
                ) && tcx.def_kind(owner) == (DefKind::Impl { of_trait: false })
                    && matches!(
                        tcx.type_of(owner).instantiate_identity().skip_norm_wip().kind(),
                        ty::Adt(adt, _)
                            if tcx.is_diagnostic_item(sym::Option, adt.did())
                                || tcx.is_diagnostic_item(sym::Result, adt.did())
                    )
            })
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
