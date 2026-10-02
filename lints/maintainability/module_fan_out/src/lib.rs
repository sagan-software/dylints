#![feature(rustc_private)]

//! A lint to bound outgoing top-level local module dependencies.
//!
//! It records resolved path and method edges, collapses each edge to its local
//! top-level module pair, and emits stable diagnostics after crate traversal.
//! Generated support crates are excluded so the threshold measures source-level
//! dependency shape rather than fixture or framework plumbing.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::collections::HashSet;

use maintainability_support::{fan_out_by_module, record_definition_edge, record_path_edge};
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, HirId, Path};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::def_id::LocalDefId;

/// Largest accepted number of distinct outgoing top-level module dependencies.
const FAN_OUT_LIMIT: usize = 7;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub MODULE_FAN_OUT,
    Warn,
    "top-level local module exceeds the dependency fan-out limit",
    ModuleFanOut,
    ModuleFanOut::default()
}

/// Stateful pass that collects resolved local module edges.
///
/// It deduplicates directed edges while the crate is visited, then sorts module
/// identities before reporting fan-out above the configured threshold. The
/// default state is empty until resolved local dependencies are observed.
#[derive(Debug, Default)]
pub struct ModuleFanOut {
    /// Deduplicated directed edges between top-level local modules.
    edges: HashSet<(LocalDefId, LocalDefId)>,
}

impl<'tcx> LateLintPass<'tcx> for ModuleFanOut {
    /// Record a resolved source path as a local module dependency.
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) {
            return;
        }
        record_path_edge(cx.tcx, path, hir_id, &mut self.edges);
    }

    /// Record resolved method calls, which do not have a callable path node.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expression: &'tcx Expr<'tcx>) {
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) {
            return;
        }
        if matches!(expression.kind, ExprKind::MethodCall(..))
            && let Some(target) = cx.typeck_results().type_dependent_def_id(expression.hir_id)
        {
            record_definition_edge(
                cx.tcx,
                expression.hir_id,
                expression.span,
                target,
                &mut self.edges,
            );
        }
    }

    /// Emit one diagnostic for each module above the fan-out limit.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Sort resolved module identities so diagnostics remain stable across runs.
        let mut fan_out: Vec<_> = fan_out_by_module(&self.edges).into_iter().collect();
        fan_out.sort_by_key(|(module, _)| cx.tcx.def_path_str(module.to_def_id()));
        // Report only modules whose distinct target count exceeds the configured limit.
        for (module, targets) in fan_out {
            if targets.len() <= FAN_OUT_LIMIT {
                continue;
            }
            let count = targets.len();
            cx.emit_span_lint(
                MODULE_FAN_OUT,
                cx.tcx.def_span(module),
                DiagDecorator(move |diagnostic| {
                    let _configured = diagnostic
                        .primary_message(format!(
                            "module depends on {count} top-level local modules, which exceeds {FAN_OUT_LIMIT}"
                        ))
                        .help("introduce a narrower facade or move orchestration to a boundary module");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
