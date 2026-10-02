#![feature(rustc_private)]

//! A lint to bound summed method complexity per implementation block.
//!
//! It accumulates Cyclomatic Complexity for source-authored inherent methods
//! that belong to the same local implementation block. The post-crate check
//! emits one stable diagnostic when the aggregate exceeds the configured
//! limit, helping maintainers split types whose methods no longer share one
//! cohesive responsibility.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::collections::HashMap;

use maintainability_support::{cyclomatic_complexity, is_macro_expansion};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

/// Largest accepted sum of method Cyclomatic Complexity in one impl block.
const METHOD_COMPLEXITY_LIMIT: u32 = 40;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub TYPE_METHOD_COMPLEXITY,
    Warn,
    "implementation block exceeds the summed method complexity limit",
    TypeMethodComplexity,
    TypeMethodComplexity::default()
}

/// Complexity accumulated for one local implementation block.
#[derive(Clone, Copy, Debug)]
struct ImplComplexity {
    /// Source span for the implementation block.
    span: Span,
    /// Number of source-authored methods.
    methods: u32,
    /// Sum of method Cyclomatic Complexity.
    complexity: u32,
}

/// Stateful pass that joins methods belonging to one implementation block.
///
/// It collects source-authored method measurements during traversal, keeps one
/// aggregate per local implementation, and emits stable diagnostics only after
/// the complete crate has been inspected. Its default state contains no entries.
#[derive(Debug, Default)]
pub struct TypeMethodComplexity {
    /// It joins methods within each local implementation block and emits one
    /// deterministic post-crate diagnostic when the aggregate exceeds its bound.
    /// Stores one aggregate for each local implementation block.
    /// Each entry records the source span, method count, and summed complexity
    /// needed for the post-crate diagnostic.
    implementations: HashMap<LocalDefId, ImplComplexity>,
}

impl<'tcx> LateLintPass<'tcx> for TypeMethodComplexity {
    /// Add one source-authored method to its enclosing implementation block.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Ignore generated code and methods that cannot belong to an inherent impl.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || !matches!(kind, FnKind::Method(..))
            || is_macro_expansion(span)
        {
            return;
        }
        let Some(impl_def_id) = cx.tcx.opt_local_parent(local_def_id) else {
            return;
        };
        if !matches!(
            cx.tcx.def_kind(impl_def_id),
            rustc_hir::def::DefKind::Impl { .. }
        ) {
            return;
        }
        // Accumulate the resolved method complexity under one stable impl identity.
        let aggregate = self
            .implementations
            .entry(impl_def_id)
            .or_insert_with(|| ImplComplexity {
                span: cx.tcx.def_span(impl_def_id),
                methods: 0,
                complexity: 0,
            });
        aggregate.methods = aggregate.methods.saturating_add(1);
        aggregate.complexity = aggregate
            .complexity
            .saturating_add(cyclomatic_complexity(body));
    }

    /// Emit one diagnostic for each implementation block above the limit.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Copy and order aggregates so hash-map traversal cannot reorder diagnostics.
        let mut aggregates: Vec<_> = self.implementations.values().copied().collect();
        aggregates.sort_by_key(|aggregate| aggregate.span.lo());
        for aggregate in aggregates {
            // Report only implementation blocks whose summed complexity exceeds the limit.
            if aggregate.complexity <= METHOD_COMPLEXITY_LIMIT {
                continue;
            }
            let methods = aggregate.methods;
            let complexity = aggregate.complexity;
            cx.emit_span_lint(
                TYPE_METHOD_COMPLEXITY,
                aggregate.span,
                DiagDecorator(move |diagnostic| {
                    let _configured = diagnostic
                        .primary_message(format!(
                            "implementation has summed method complexity {complexity} across {methods} methods, which exceeds {METHOD_COMPLEXITY_LIMIT}"
                        ))
                        .help("split unrelated responsibilities into separate types or impl blocks");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
