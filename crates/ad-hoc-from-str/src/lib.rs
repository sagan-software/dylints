#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc string parser functions.
//!
//! It finds free functions and inherent associated functions that parse a
//! `&str` into a `Result` through a parser-shaped name instead of implementing
//! `FromStr`. Resolved types decide whether a `FromStr` implementation is legal
//! and new: the target must be a local type that does not borrow from the input
//! and does not already implement `FromStr`.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, FnDecl, ImplItem, ImplItemImplKind, ImplItemKind, Mutability, intravisit::FnKind,
};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol, def_id::LocalDefId, sym};
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_FROM_STR,
    Warn,
    "ad hoc string parser could be `FromStr`",
    AdHocFromStr
}

impl<'tcx> LateLintPass<'tcx> for AdHocFromStr {
    /// Check one free function.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        _body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if let FnKind::ItemFn(ident, ..) = kind {
            check_candidate(cx, ident.name, span, local_def_id);
        }
    }

    /// Check one inherent associated function; trait items have names fixed by their trait.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let def_id = item.owner_id.def_id;
        if matches!(item.kind, ImplItemKind::Fn(..))
            && matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            && !cx.tcx.associated_item(def_id).is_method()
        {
            check_candidate(cx, item.ident.name, item.span, def_id);
        }
    }
}

/// Report one function whose name and signature fit a new `FromStr` implementation.
fn check_candidate(cx: &LateContext<'_>, name: Symbol, span: Span, local_def_id: LocalDefId) {
    // Resolve parser vocabulary and the target type before applying ownership rules.
    let Some((_input, target)) = parser_signature(cx, name, local_def_id) else {
        return;
    };
    // `FromStr` needs a local target that cannot borrow from the input and lacks an impl.
    if !is_valid_from_str_conversion(cx, target) {
        return;
    }

    cx.emit_span_lint(
        AD_HOC_FROM_STR,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(format!("function `{name}` looks like a string parser"));
            let _ = diag.help(format!(
                "implement `std::str::FromStr for {target}` when the parser has one canonical meaning"
            ));
        }),
    );
}

/// Return the input and target types of a named string parser.
fn parser_signature<'tcx>(
    cx: &LateContext<'tcx>,
    name: Symbol,
    local_def_id: LocalDefId,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    // Reject names outside parser vocabulary before resolving the signature.
    if !is_parser_name(name.as_str()) {
        return None;
    }
    let fn_sig = cx.tcx.instantiate_bound_regions_with_erased(
        cx.tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    );
    let ([input], ty::Adt(result, args)) = (fn_sig.inputs(), fn_sig.output().kind()) else {
        return None;
    };
    // Keep only `&str -> Result<T, E>` signatures with an available target type.
    if !matches!(input.kind(), ty::Ref(_, inner, Mutability::Not) if inner.is_str())
        || !cx.tcx.is_diagnostic_item(sym::Result, result.did())
    {
        return None;
    }
    Some((*input, args.types().next()?))
}

/// Return whether a parser target is local, owned, and not already `FromStr`.
fn is_valid_from_str_conversion<'tcx>(cx: &LateContext<'tcx>, target: Ty<'tcx>) -> bool {
    is_local_owned_adt(target) && !implements_from_str(cx, target)
}

/// Return whether the name reads as a canonical parser without a policy marker.
fn is_parser_name(name: &str) -> bool {
    (name.starts_with("parse_") || name.starts_with("from_str"))
        && ![
            "_and_",
            "_lenient",
            "_lossy",
            "_or_",
            "_strict",
            "_unchecked",
            "_with_",
        ]
        .iter()
        .any(|marker| name.contains(marker))
}

/// Return whether the type is a local struct, enum, or union without lifetime arguments.
///
/// `FromStr::from_str` cannot tie its output to the input string, so a target
/// with a lifetime cannot follow the suggestion.
fn is_local_owned_adt(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local())
        && !ty.walk().any(|argument| argument.as_region().is_some())
}

/// Return whether the target may already implement `FromStr` in the item's environment.
fn implements_from_str<'tcx>(cx: &LateContext<'tcx>, target: Ty<'tcx>) -> bool {
    // `FromStr` itself has no diagnostic item, so resolve it through its `from_str` method.
    let from_str = cx
        .tcx
        .get_diagnostic_item(Symbol::intern("from_str_method"))
        .map(|method| cx.tcx.parent(method));
    from_str.is_none_or(|from_str| {
        cx.tcx
            .infer_ctxt()
            .build(cx.typing_mode())
            .type_implements_trait(from_str, [target], cx.param_env)
            .may_apply()
    })
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
