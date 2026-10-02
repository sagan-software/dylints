#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc string parser functions.
//!
//! It finds source-authored functions and inherent methods that parse strings
//! through a repeated conversion-shaped API instead of implementing `FromStr`.
//! The check relies on resolved signatures and body structure, reports a
//! focused migration hint, and leaves functions with unrelated side effects
//! or incompatible return types untouched.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, FnDecl, ImplItem, ImplItemImplKind, ImplItemKind, ImplicitSelfKind, Mutability,
    intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_FROM_STR,
    Warn,
    "ad hoc string parser could be `FromStr`",
    AdHocFromStr
}

impl<'tcx> LateLintPass<'tcx> for AdHocFromStr {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        _body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        let FnKind::ItemFn(ident, ..) = kind else {
            return;
        };

        let name = ident.name.to_ident_string();
        check_candidate(cx, &name, None, span, local_def_id);
    }

    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(sig, _) = item.kind else {
            return;
        };
        let name = item.ident.name.to_ident_string();

        // Leave canonical `FromStr::from_str` implementations to the trait contract.
        if standard_from_str_impl(cx, item, &name) {
            return;
        }

        check_candidate(cx, &name, Some(sig.decl), item.span, item.owner_id.def_id);
    }
}

/// Check candidate for this lint.
fn check_candidate(
    cx: &LateContext<'_>,
    name: &str,
    decl: Option<&FnDecl<'_>>,
    span: Span,
    local_def_id: LocalDefId,
) {
    // Require parser vocabulary and a string-to-result signature together.
    if !parser_name(name) || !one_string_input_result(cx, decl, local_def_id) {
        return;
    }

    // Recommend the standard parsing trait for the detected canonical shape.
    emit_span_lint_with_help(
        cx,
        AD_HOC_FROM_STR,
        span,
        format!("function `{name}` looks like a string parser"),
        "implement `std::str::FromStr` when the parser has one canonical meaning",
    );
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return the parser name.
fn parser_name(name: &str) -> bool {
    (name.starts_with("parse_") || name.starts_with("from_str"))
        && !domain_or_side_effect_policy_name(name)
}

/// Return the domain or side effect policy name.
fn domain_or_side_effect_policy_name(name: &str) -> bool {
    [
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

/// Helper for one string input result analysis.
fn one_string_input_result(
    cx: &LateContext<'_>,
    decl: Option<&FnDecl<'_>>,
    local_def_id: LocalDefId,
) -> bool {
    if decl.is_some_and(|decl| decl.implicit_self() != ImplicitSelfKind::None) {
        return false;
    }

    let fn_sig = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_binder();
    let [input] = fn_sig.inputs() else {
        return false;
    };

    // Use lowered rustc types so aliases like `type Raw<'a> = &'a str` and qualified
    // `std::result::Result` paths are handled the same as their canonical spelling.
    str_ref_ty(*input) && result_return(cx, fn_sig.output())
}

/// Return type information for str ref.
fn str_ref_ty(ty: Ty<'_>) -> bool {
    let ty::Ref(_, inner, Mutability::Not) = ty.kind() else {
        return false;
    };

    matches!(inner.kind(), ty::Str)
}

/// Helper for result return analysis.
fn result_return(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, args) = ty.kind() else {
        return false;
    };

    // Require a meaningful parse target so result-shaped status helpers like `Result<(), E>` stay
    // out of the trait-convention lint.
    cx.tcx.is_diagnostic_item(sym::Result, adt.did())
        && args.len() == 2
        && parse_target_ty(args.type_at(0))
}

/// Parse target ty from source text.
fn parse_target_ty(ty: Ty<'_>) -> bool {
    !matches!(ty.kind(), ty::Tuple(fields) if fields.is_empty())
        && !matches!(ty.kind(), ty::Never | ty::Ref(..))
}

/// Return whether this is the standard from str impl shape.
fn standard_from_str_impl(cx: &LateContext<'_>, item: &ImplItem<'_>, name: &str) -> bool {
    if name != "from_str" || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Resolve the parent impl's trait so `use std::str::FromStr as Parse` and qualified paths
    // are skipped without relying on source spelling.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    from_str_trait(cx, trait_def_id)
}

/// Helper for from str trait analysis.
fn from_str_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    matches!(
        cx.tcx.def_path_str(trait_def_id).as_str(),
        "core::str::traits::FromStr" | "std::str::FromStr"
    )
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
