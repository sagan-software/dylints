#![feature(rustc_private)]

//! A lint to detect cycles between top-level local modules.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::collections::HashSet;

use maintainability_support::{dependency_cycles, record_definition_edge, record_path_edge};
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, HirId, Path};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::def_id::LocalDefId;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub MODULE_DEPENDENCY_CYCLE,
    Warn,
    "top-level local modules form a dependency cycle",
    ModuleDependencyCycle,
    ModuleDependencyCycle::default()
}

/// Stateful pass that collects resolved local module edges.
///
/// It records each source-resolved dependency during traversal, deduplicates
/// directed pairs, and emits stable cycle diagnostics after the complete crate
/// has been visited. The default state contains no edges until paths are seen.
#[derive(Debug, Default)]
pub struct ModuleDependencyCycle {
    /// Stores deduplicated directed edges between top-level local modules.
    /// Traversal collects each resolved dependency direction, then the
    /// post-crate pass converts the stable edge set into cycle diagnostics.
    /// The state remains empty for crates without local module references.
    /// Callers construct the pass with `Default` and inspect only emitted lints.
    edges: HashSet<(LocalDefId, LocalDefId)>,
}

impl<'tcx> LateLintPass<'tcx> for ModuleDependencyCycle {
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

    /// Emit one deterministic diagnostic for each strongly connected component.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Canonicalize each component before selecting its stable primary span.
        let mut cycles: Vec<_> = dependency_cycles(&self.edges)
            .into_iter()
            .filter_map(|cycle| {
                let mut modules: Vec<_> = cycle.into_iter().collect();
                modules.sort_by_key(|module| cx.tcx.def_path_str(module.to_def_id()));
                let primary = modules.first().copied()?;
                Some((primary, modules))
            })
            .collect();
        cycles.sort_by_key(|(primary, _)| cx.tcx.def_path_str(primary.to_def_id()));
        // Emit components in path order so UI output stays reproducible across traversals.
        for (primary, modules) in cycles {
            let names = modules
                .iter()
                .map(|module| cx.tcx.def_path_str(module.to_def_id()))
                .collect::<Vec<_>>()
                .join(", ");
            cx.emit_span_lint(
                MODULE_DEPENDENCY_CYCLE,
                cx.tcx.def_span(primary),
                DiagDecorator(move |diagnostic| {
                    let _configured = diagnostic
                        .primary_message(format!("module dependency cycle contains: {names}"))
                        .help("move the shared contract inward or reverse one dependency");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
