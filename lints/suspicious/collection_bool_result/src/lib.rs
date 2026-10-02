#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc type variants"
)]
#![warn(unused_extern_crates)]

//! A lint to check for return types pairing collections with booleans.
//!
//! It examines source-authored function return types and recognizes tuples or
//! structs that expose a collection beside a boolean status value. The rule
//! reports ambiguous success indicators while preserving APIs whose resolved
//! types and names document an intentional pair of independent results.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{Body, ExprKind, FnDecl, FnRetTy, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub COLLECTION_BOOL_RESULT,
    Warn,
    "return type pairs a collection with an independent boolean",
    CollectionBoolResult
}

impl<'tcx> LateLintPass<'tcx> for CollectionBoolResult {
    /// Check function return values for collection and boolean pairs.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Analyze named functions and methods because closures have no stable API contract.
        if matches!(kind, FnKind::Closure) {
            return;
        }
        let FnRetTy::Return(output) = decl.output else {
            return;
        };
        // Resolve aliases before looking through the returned type structure. An `async fn`
        // returns an opaque future, so read the declared output from its coroutine instead.
        let return_ty = async_fn_output(cx, body).unwrap_or_else(|| {
            cx.tcx
                .fn_sig(local_def_id)
                .instantiate_identity()
                .skip_norm_wip()
                .output()
                .skip_binder()
        });
        if !contains_collection_bool_pair(cx, return_ty) {
            return;
        }

        cx.emit_span_lint(
            COLLECTION_BOOL_RESULT,
            output.span,
            DiagDecorator(|diag| {
                let _ = diag
                    .primary_message("return type pairs a collection with an independent boolean");
                let _ = diag.help(
                    "use an enum or a result type that cannot represent contradictory states",
                );
            }),
        );
    }
}

/// Return the declared output type of an `async fn` from its coroutine body.
fn async_fn_output<'tcx>(cx: &LateContext<'tcx>, body: &Body<'tcx>) -> Option<Ty<'tcx>> {
    // Only an `async fn` has a closure as its whole body; its coroutine returns the output.
    if let ExprKind::Closure(_) = body.value.kind
        && let ty::Coroutine(_, args) = cx.typeck_results().expr_ty(body.value).kind()
    {
        Some(args.as_coroutine().return_ty())
    } else {
        None
    }
}

/// Search transparent result wrappers for a two-element collection and boolean tuple.
fn contains_collection_bool_pair(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Tuple(elements) if elements.len() == 2 => {
            (is_collection(cx, elements[0]) && elements[1].is_bool())
                || (elements[0].is_bool() && is_collection(cx, elements[1]))
        }
        ty::Adt(adt, args)
            if cx.tcx.is_diagnostic_item(sym::Result, adt.did())
                || cx.tcx.is_diagnostic_item(sym::Option, adt.did()) =>
        {
            contains_collection_bool_pair(cx, args.type_at(0))
        }
        _ => false,
    }
}

/// Return whether the type is an array or a standard collection, matched by diagnostic item.
fn is_collection(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Array(..) | ty::Slice(_) => true,
        ty::Adt(adt, _) => cx.tcx.get_diagnostic_name(adt.did()).is_some_and(|name| {
            matches!(
                name.as_str(),
                "BTreeMap" | "BTreeSet" | "HashMap" | "HashSet" | "Vec" | "VecDeque"
            )
        }),
        _ => false,
    }
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
