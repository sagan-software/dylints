#![feature(rustc_private)]

//! A lint to bound inherent methods per local type.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::HashMap;

use maintainability_support::is_macro_expansion;
use rustc_errors::DiagDecorator;
use rustc_hir::{ImplItem, ImplItemImplKind, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::def_id::LocalDefId;

/// Largest accepted number of source-authored inherent methods per local type.
const METHOD_COUNT_LIMIT: u32 = 20;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub IMPL_METHOD_COUNT,
    Warn,
    "local type exceeds the inherent-method limit",
    ImplMethodCount,
    ImplMethodCount::default()
}

/// Stateful pass that joins inherent methods across every impl for one type.
///
/// It counts source-authored inherent methods, keeps separate impl blocks under
/// one local type identity, and emits one deterministic diagnostic after crate
/// traversal. The default state is empty until eligible methods are observed.
#[derive(Debug, Default)]
pub struct ImplMethodCount {
    /// Counts source-authored inherent methods keyed by local type definition.
    /// The map is populated during traversal and read after the complete crate
    /// is visited, allowing separate inherent impl blocks to share one limit.
    /// A missing key means that no eligible method has been observed yet.
    /// The pass is created with `Default` and exposes its result through lints.
    methods_by_type: HashMap<LocalDefId, u32>,
}

impl<'tcx> LateLintPass<'tcx> for ImplMethodCount {
    /// Add one source-authored inherent method to its local type.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Trait and generated methods do not enlarge the type's inherent interface.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            || !matches!(item.kind, ImplItemKind::Fn(..))
            || is_macro_expansion(item.span)
        {
            return;
        }
        let Some(type_id) = inherent_impl_type(cx, item) else {
            return;
        };
        // Join separate inherent impl blocks under the defining local type.
        let methods = self.methods_by_type.entry(type_id).or_default();
        *methods = methods.saturating_add(1);
    }

    /// Emit one diagnostic for each local type above the method limit.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Stabilize diagnostic order across hash-map iteration and compiler runs.
        let mut counts: Vec<_> = self.methods_by_type.iter().collect();
        counts.sort_by_key(|(type_id, _)| cx.tcx.def_path_str(type_id.to_def_id()));
        for (&type_id, &method_count) in counts {
            if method_count <= METHOD_COUNT_LIMIT {
                continue;
            }
            // Anchor the aggregate warning on the type definition shared by its impl blocks.
            cx.emit_span_lint(
                IMPL_METHOD_COUNT,
                cx.tcx.def_span(type_id),
                DiagDecorator(move |diagnostic| {
                    let _configured = diagnostic
                        .primary_message(format!(
                            "local type has {method_count} inherent methods, which exceeds {METHOD_COUNT_LIMIT}"
                        ))
                        .help(
                            "extract an independent capability when its state and invariants can stand alone",
                        );
                }),
            );
        }
    }
}

/// Resolve an inherent impl item to its local algebraic data type.
fn inherent_impl_type(cx: &LateContext<'_>, item: &ImplItem<'_>) -> Option<LocalDefId> {
    // Resolve through the parent impl because the method's owner is not the ADT.
    let impl_id = cx.tcx.opt_local_parent(item.owner_id.def_id)?;
    let self_type = cx
        .tcx
        .type_of(impl_id)
        .instantiate_identity()
        .skip_norm_wip();
    let ty::Adt(definition, _) = self_type.kind() else {
        return None;
    };
    definition.did().as_local()
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
