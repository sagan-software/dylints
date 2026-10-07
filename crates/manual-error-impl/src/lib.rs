#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for manual `Error` and `Display` implementations that could
//! use thiserror.
//!
//! It pairs `Display` and `Error` implementations by the resolved local type, so
//! types with the same name in different modules stay separate. A `Display`
//! implementation qualifies when its `fmt` body is one `write!` call, and an
//! `Error` implementation qualifies when it has no items. Both traits are
//! resolved through diagnostic items, so a local trait named `Display` or
//! `Error` does not count.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::HashMap;

use rustc_errors::DiagDecorator;
use rustc_hir::{ExprKind, Impl, ImplItemKind, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol, def_id::DefId, sym};

#[cfg(test)]
use thiserror as _;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ERROR_IMPL,
    Warn,
    "manual `Error` and `Display` implementations could use `thiserror`",
    ManualErrorImpl,
    ManualErrorImpl::default()
}

/// Stateful pass that pairs `Display` and `Error` implementations by local type.
#[derive(Default)]
struct ManualErrorImpl {
    /// Evidence gathered for each local type.
    candidates: HashMap<DefId, ErrorImplCandidate>,
}

/// Evidence gathered for one local type.
#[derive(Default)]
struct ErrorImplCandidate {
    /// Span of a `Display` implementation whose body is one `write!` call.
    simple_display: Option<Span>,
    /// Whether an `Error` implementation without items exists.
    has_empty_error_impl: bool,
}

impl<'tcx> LateLintPass<'tcx> for ManualErrorImpl {
    /// Record one `Display` or `Error` implementation for a local, non-generic type.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Filter unsupported implementations before classifying the resolved trait.
        let Some(adt_id) = supported_adt(cx, item) else {
            return;
        };
        let ItemKind::Impl(Impl {
            of_trait: Some(_),
            items,
            ..
        }) = item.kind
        else {
            return;
        };
        let trait_ref = cx.tcx.impl_trait_ref(item.owner_id).skip_binder();

        // Classify the implementation by its resolved trait.
        if cx.tcx.is_diagnostic_item(sym::Display, trait_ref.def_id) {
            let is_simple_display = is_simple_display_impl(cx, items);
            if is_simple_display {
                self.candidates.entry(adt_id).or_default().simple_display = Some(item.span);
            }
        } else if cx
            .tcx
            .is_diagnostic_item(Symbol::intern("Error"), trait_ref.def_id)
            && items.is_empty()
        {
            self.candidates
                .entry(adt_id)
                .or_default()
                .has_empty_error_impl = true;
        }
    }

    /// Emit one diagnostic for each type with both qualifying implementations.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Report in source order so output does not depend on hash-map iteration.
        let mut spans: Vec<Span> = self
            .candidates
            .values()
            .filter(|candidate| candidate.has_empty_error_impl)
            .filter_map(|candidate| candidate.simple_display)
            .collect();
        spans.sort_by_key(|span| span.lo());
        for span in spans {
            cx.emit_span_lint(
                MANUAL_ERROR_IMPL,
                span,
                DiagDecorator(|diag| {
                    let _ = diag.primary_message(
                        "manual `Error` and `Display` implementations look derivable",
                    );
                    let _ = diag.help(
                        "use `#[derive(thiserror::Error, Debug)]` with a `#[error(\"...\")]` message",
                    );
                }),
            );
        }
    }
}

/// Return whether an implementation has exactly one qualifying `fmt` method.
fn is_simple_display_impl(cx: &LateContext<'_>, items: &[rustc_hir::ImplItemId]) -> bool {
    let Some(fmt_id) = items.first() else {
        return false;
    };
    items.len() == 1 && is_single_write(cx, cx.tcx.hir_impl_item(*fmt_id).kind)
}

/// Return the local type implemented by a source-authored, non-generic impl.
fn supported_adt(cx: &LateContext<'_>, item: &Item<'_>) -> Option<DefId> {
    let ItemKind::Impl(Impl {
        of_trait: Some(_), ..
    }) = item.kind
    else {
        return None;
    };
    let trait_ref = cx.tcx.impl_trait_ref(item.owner_id).skip_binder();
    let ty::Adt(adt, args) = trait_ref.self_ty().kind() else {
        return None;
    };
    // Generated implementations do not represent source-level migration candidates.
    let is_source_authored = !item.span.from_expansion();
    // The lint pairs only local implementations with fully concrete types.
    let local_type = adt.did().is_local();
    let concrete_type = args.non_erasable_generics().next().is_none();
    (is_source_authored && local_type && concrete_type).then_some(adt.did())
}

/// Return whether a `fmt` method body is exactly one `write!` call.
fn is_single_write(cx: &LateContext<'_>, kind: ImplItemKind<'_>) -> bool {
    let ImplItemKind::Fn(_, body_id) = kind else {
        return false;
    };
    let ExprKind::Block(block, None) = cx.tcx.hir_body(body_id).value.kind else {
        return false;
    };
    // The block's only expression must come from the standard `write!` macro.
    block.stmts.is_empty()
        && block.expr.is_some_and(|expr| {
            let expansion = expr.span.ctxt().outer_expn_data();
            expansion
                .macro_def_id
                .is_some_and(|macro_id| cx.tcx.is_diagnostic_item(sym::write_macro, macro_id))
                && !expansion.call_site.from_expansion()
        })
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
