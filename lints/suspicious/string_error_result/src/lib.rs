#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for Result error types erased to String.
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
use rustc_hir::{
    Body, FnDecl, FnRetTy, GenericArg, GenericBound, LangItem, OpaqueTyOrigin, QPath, Ty as HirTy,
    TyKind, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub STRING_ERROR_RESULT,
    Warn,
    "`Result` error type erased to `String`",
    StringErrorResult
}

impl<'tcx> LateLintPass<'tcx> for StringErrorResult {
    /// Check fn for this lint.
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

        // An `async fn` returns an opaque future; compare its written `Output` type instead.
        if let Some(async_output) = async_fn_output(cx, output, body) {
            check_result_string_error_ty(cx, async_output.0, async_output.1);
            return;
        }

        // Resolve aliases before comparing the returned error type.
        let output_ty = cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .output()
            .skip_binder();
        check_result_string_error_ty(cx, output_ty, output);
    }
}

/// Return the semantic and written `Output` type of an `async fn`.
///
/// rustc lowers `async fn f() -> T` to an opaque `impl Future<Output = T>` whose
/// bound keeps the written `T`, and to a coroutine body whose return type is `T`.
fn async_fn_output<'tcx>(
    cx: &LateContext<'tcx>,
    output: &'tcx HirTy<'tcx>,
    body: &'tcx Body<'tcx>,
) -> Option<(Ty<'tcx>, &'tcx HirTy<'tcx>)> {
    // The opaque type's `Future<Output = T>` bound carries the written `T`.
    if let TyKind::OpaqueDef(opaque) = output.kind
        && let OpaqueTyOrigin::AsyncFn { .. } = opaque.origin
        && let Some(written) = opaque
            .bounds
            .iter()
            .filter_map(GenericBound::trait_ref)
            .filter_map(|trait_ref| trait_ref.path.segments.last()?.args)
            .flat_map(|args| args.constraints)
            .find_map(|constraint| constraint.ty())
        && let ty::Coroutine(_, args) = cx.typeck_results().expr_ty(body.value).kind()
    {
        Some((args.as_coroutine().return_ty(), written))
    } else {
        None
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to avoid depending on Clippy utilities.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for result string error analysis.
fn result_string_error<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    // Require the standard Result diagnostic item before inspecting generic arguments.
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };
    if !cx.tcx.is_diagnostic_item(sym::Result, adt.did()) {
        return None;
    }

    // Return only the error argument when it resolves to alloc::String.
    let error_ty = args.type_at(1);
    is_string_ty(cx, error_ty).then_some(error_ty)
}

/// Check result string error ty for this lint.
fn check_result_string_error_ty<'tcx, I>(
    cx: &LateContext<'tcx>,
    semantic_ty: Ty<'tcx>,
    hir_ty: &'tcx HirTy<'tcx, I>,
) {
    if result_string_error(cx, semantic_ty).is_some() {
        emit_span_lint_with_help(
            cx,
            STRING_ERROR_RESULT,
            error_arg_span(hir_ty),
            "`Result` uses `String` as its error type",
            "use a concrete error type or a focused `thiserror` enum",
        );
    }

    check_nested_type_args(cx, semantic_ty, hir_ty);
}

/// Check nested type args for this lint.
fn check_nested_type_args<'tcx, I>(
    cx: &LateContext<'tcx>,
    semantic_ty: Ty<'tcx>,
    hir_ty: &'tcx HirTy<'tcx, I>,
) {
    let ty::Adt(_, args) = semantic_ty.kind() else {
        return;
    };
    let Some(hir_args) = path_type_args(hir_ty) else {
        return;
    };

    // Pair type arguments semantically so lifetimes and const generics cannot
    // shift a HIR type onto a non-type rustc generic argument.
    let hir_types = hir_args.args.iter().filter_map(|arg| match arg {
        GenericArg::Type(ty) => Some(*ty),
        GenericArg::Lifetime(_) | GenericArg::Const(_) | GenericArg::Infer(_) => None,
    });
    for (semantic_arg, hir_arg_ty) in args.types().zip(hir_types) {
        check_result_string_error_ty(cx, semantic_arg, hir_arg_ty);
    }
}

/// Return whether the type is the standard `String`, however it is spelled.
fn is_string_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if cx.tcx.is_lang_item(adt.did(), LangItem::String))
}

/// Return the span for error arg.
fn error_arg_span<I>(ty: &HirTy<'_, I>) -> Span {
    // Prefer the written error argument and fall back to the complete return type.
    let Some(args) = path_type_args(ty) else {
        return ty.span;
    };
    let Some(GenericArg::Type(error_ty)) = args.args.get(1) else {
        return ty.span;
    };

    error_ty.span
}

/// Helper for path type args analysis.
fn path_type_args<'hir, I>(ty: &HirTy<'hir, I>) -> Option<&'hir rustc_hir::GenericArgs<'hir>> {
    let TyKind::Path(QPath::Resolved(_, path)) = ty.kind else {
        return None;
    };

    path.segments.last().and_then(|segment| segment.args)
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
