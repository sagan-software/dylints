#![feature(rustc_private)]

//! A lint to detect disconnected field-use clusters in local types.
//!
//! It records direct receiver-field and receiver-method relationships for
//! source-authored inherent methods, builds a method graph, and reports types
//! with multiple substantial connected components. The thresholds keep small
//! or weakly observed types outside the decision while preserving deterministic
//! diagnostics for larger source-level designs.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::{HashMap, HashSet};

use maintainability_support::is_macro_expansion;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl, HirId, PatKind,
    def::Res,
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::{
    Span, Symbol,
    def_id::{DefId, LocalDefId},
};

/// Smallest method population worth a type-level cohesion judgment.
const MINIMUM_METHODS: usize = 6;
/// Smallest used-field population worth a type-level cohesion judgment.
const MINIMUM_FIELDS: usize = 4;
/// Smallest method population treated as a substantial component.
const MINIMUM_COMPONENT_METHODS: usize = 2;
/// Smallest field population treated as a substantial component.
const MINIMUM_COMPONENT_FIELDS: usize = 2;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub FIELD_USAGE_COHESION,
    Warn,
    "local type has disconnected field-use clusters",
    FieldUsageCohesion,
    FieldUsageCohesion::default()
}

/// Stateful pass that joins field usage across every inherent impl for one type.
///
/// It records source-authored receiver methods, builds a field-and-call graph,
/// and emits one deterministic diagnostic after all methods for each local type
/// have been visited. The default state contains no measured methods or fields.
#[derive(Debug, Default)]
pub struct FieldUsageCohesion {
    /// Groups measured receiver methods by their local algebraic data type.
    /// Each method stores direct field uses and calls to other receiver methods
    /// so the completed graph can be analyzed after crate traversal. Empty
    /// groups are retained only while the current crate is being visited.
    usage_by_type: HashMap<LocalDefId, HashMap<LocalDefId, MethodUsage>>,
}

/// Direct state and method dependencies observed in one receiver method.
#[derive(Debug, Default)]
struct MethodUsage {
    /// Fields accessed directly through `self`.
    fields: HashSet<Symbol>,
    /// Local methods called directly through `self`.
    calls: HashSet<LocalDefId>,
}

impl<'tcx> LateLintPass<'tcx> for FieldUsageCohesion {
    /// Record direct field and method uses in one source-authored receiver method.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        declaration: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if let Some((type_id, usage)) =
            method_usage(cx, kind, declaration, body, span, local_def_id)
        {
            // Keep one resolved usage record per source-authored receiver method.
            let _previous_usage = self
                .usage_by_type
                .entry(type_id)
                .or_default()
                .insert(local_def_id, usage);
        }
    }

    /// Emit one diagnostic for each type with multiple substantial state clusters.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Stabilize type-level diagnostics across hash-map iteration and compiler runs.
        let mut type_usage: Vec<_> = self.usage_by_type.iter().collect();
        type_usage.sort_by_key(|(type_id, _)| cx.tcx.def_path_str(type_id.to_def_id()));
        // Evaluate each type only after all source-authored receiver methods are recorded.
        for (&type_id, methods) in type_usage {
            let Some(summary) = cohesion_summary(methods) else {
                continue;
            };
            let substantial_components = summary.substantial_components;
            let measured_methods = summary.measured_methods;
            let used_fields = summary.used_fields;
            let largest_component_percent = summary.largest_component_percent;
            // Anchor the aggregate evidence on the type shared by all measured methods.
            cx.emit_span_lint(
                FIELD_USAGE_COHESION,
                cx.tcx.def_span(type_id),
                DiagDecorator(move |diagnostic| {
                    let _configured = diagnostic
                        .primary_message(format!(
                            "type has {substantial_components} substantial field-use clusters across {measured_methods} methods and {used_fields} fields; the largest cluster contains {largest_component_percent}% of measured methods"
                        ))
                        .help(
                            "extract a field cluster only when it has an independent invariant and lifecycle",
                        );
                }),
            );
        }
    }
}

/// Collects field and receiver-call usage for one eligible inherent method.
fn method_usage<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    declaration: &'tcx FnDecl<'tcx>,
    body: &'tcx Body<'tcx>,
    span: Span,
    local_def_id: LocalDefId,
) -> Option<(LocalDefId, MethodUsage)> {
    // Keep only source-authored receiver methods from inherent implementations.
    let eligible_method = (!dylint_support::is_internal_support_crate(
        cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
    ) && matches!(kind, FnKind::Method(..))
        && declaration.implicit_self().has_implicit_self()
        && !is_macro_expansion(span))
    .then_some(());
    eligible_method
        .map(|()| cx.tcx.local_parent(local_def_id))
        .filter(|impl_id| {
            matches!(
                cx.tcx.def_kind(*impl_id),
                rustc_hir::def::DefKind::Impl { .. }
            )
        })
        .filter(|impl_id| cx.tcx.impl_opt_trait_ref(*impl_id).is_none())
        // Resolve the receiver type before walking expressions in the method body.
        .and_then(|impl_id| inherent_impl_type(cx, impl_id))
        .zip(self_binding(body))
        .map(|(type_id, self_id)| {
            // Resolve relationships while this method's type-checking context is active.
            let mut visitor = MethodUsageVisitor {
                cx,
                self_id,
                usage: MethodUsage::default(),
            };
            visitor.visit_expr(body.value);
            (type_id, visitor.usage)
        })
}

/// Visitor that records direct `self.field` and `self.method()` relationships.
struct MethodUsageVisitor<'a, 'tcx> {
    /// Lint context used for local path and method resolution.
    cx: &'a LateContext<'tcx>,
    /// HIR binding introduced by the receiver parameter.
    self_id: HirId,
    /// Relationships accumulated for the current method.
    usage: MethodUsage,
}

impl<'tcx> Visitor<'tcx> for MethodUsageVisitor<'_, 'tcx> {
    /// Enter closure bodies, which share the method's type-checking results and receiver.
    type NestedFilter = OnlyBodies;

    /// Provide the compiler context needed to enter closure bodies.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Record direct receiver relationships, excluding macro-generated implementation details.
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        // Generated accesses should not create or join source-level state clusters, but
        // source-authored macro arguments such as `format!("{}", self.name)` still do.
        if !is_macro_expansion(expression.span) {
            self.record_relationship(expression);
        }
        intravisit::walk_expr(self, expression);
    }
}

impl MethodUsageVisitor<'_, '_> {
    /// Record one `self.field` use or one resolved local `self.method()` call.
    fn record_relationship(&mut self, expression: &Expr<'_>) {
        if let ExprKind::Field(receiver, field) = expression.kind
            && self.is_self(receiver)
        {
            let _is_new_field = self.usage.fields.insert(field.name);
        } else if let ExprKind::MethodCall(_, receiver, _, _) = expression.kind
            && self.is_self(receiver)
            && let Some(target) = self
                .cx
                .typeck_results()
                .type_dependent_def_id(expression.hir_id)
                .and_then(DefId::as_local)
        {
            let _is_new_call = self.usage.calls.insert(target);
        }
    }

    /// Return whether an expression resolves to the current method's receiver binding.
    fn is_self(&self, expression: &Expr<'_>) -> bool {
        // Syntactic names are insufficient because a nested binding may shadow `self`-like paths.
        let ExprKind::Path(path) = expression.kind else {
            return false;
        };
        matches!(
            self.cx
                .typeck_results()
                .qpath_res(&path, expression.hir_id),
            Res::Local(binding) if binding == self.self_id
        )
    }
}

/// Resolve an inherent impl item to its local algebraic data type.
fn inherent_impl_type(cx: &LateContext<'_>, impl_id: LocalDefId) -> Option<LocalDefId> {
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

/// Return the receiver binding from a body whose signature has implicit self.
fn self_binding(body: &Body<'_>) -> Option<HirId> {
    // An implicit `self` parameter always lowers to a plain binding pattern.
    let receiver = body.params.first()?;
    let PatKind::Binding(_, binding, _, _) = receiver.pat.kind else {
        return None;
    };
    Some(binding)
}

/// Cohesion evidence emitted for one qualifying local type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CohesionSummary {
    /// Number of disconnected components with several methods and fields.
    substantial_components: usize,
    /// Number of receiver methods participating in the measured graph.
    measured_methods: usize,
    /// Number of fields participating in the measured graph.
    used_fields: usize,
    /// Share of measured methods in the largest connected component.
    largest_component_percent: usize,
}

/// Summarize disconnected method-field components when the evidence is substantial.
fn cohesion_summary(methods: &HashMap<LocalDefId, MethodUsage>) -> Option<CohesionSummary> {
    // Measure only methods that contribute fields or known receiver calls.
    let measured = measured_methods(methods);
    let used_fields: HashSet<_> = measured
        .iter()
        .flat_map(|(_, usage)| usage.fields.iter().copied())
        .collect();
    if measured.len() < MINIMUM_METHODS || used_fields.len() < MINIMUM_FIELDS {
        return None;
    }

    // Build the graph before applying component-size thresholds.
    let adjacency = method_adjacency(&measured);
    let components = connected_components(measured.len(), &adjacency);
    // Require two substantial components before producing a type-level finding.
    // Count components that contain enough methods and distinct fields to be meaningful.
    let substantial_components = components
        .iter()
        .filter(|component| {
            let fields: HashSet<_> = component
                .iter()
                .filter_map(|&index| measured.get(index))
                .flat_map(|(_, usage)| usage.fields.iter().copied())
                .collect();
            component.len() >= MINIMUM_COMPONENT_METHODS && fields.len() >= MINIMUM_COMPONENT_FIELDS
        })
        .count();
    if substantial_components < 2 {
        return None;
    }

    let largest_component = components.iter().map(Vec::len).max().unwrap_or_default();
    Some(CohesionSummary {
        substantial_components,
        measured_methods: measured.len(),
        used_fields: used_fields.len(),
        largest_component_percent: largest_component.saturating_mul(100) / measured.len(),
    })
}

/// Keep methods that use fields directly or delegate to another measured method.
fn measured_methods(
    methods: &HashMap<LocalDefId, MethodUsage>,
) -> Vec<(&LocalDefId, &MethodUsage)> {
    methods
        .iter()
        .filter(|(_, usage)| {
            !usage.fields.is_empty()
                || usage
                    .calls
                    .iter()
                    .any(|target| methods.contains_key(target))
        })
        .collect()
}

/// Build an undirected method graph from shared fields and direct receiver calls.
fn method_adjacency(measured: &[(&LocalDefId, &MethodUsage)]) -> HashMap<usize, HashSet<usize>> {
    // Methods without an edge have no entry; component discovery treats them as isolated.
    let mut adjacency: HashMap<usize, HashSet<usize>> = HashMap::new();
    for (left, &(left_id, left_usage)) in measured.iter().enumerate() {
        // Compare each unordered method pair once and add a symmetric edge.
        for (right, &(right_id, right_usage)) in measured.iter().enumerate().skip(left + 1) {
            // Join methods that share state or directly delegate to one another.
            let shares_field = !left_usage.fields.is_disjoint(&right_usage.fields);
            let left_calls_right = left_usage.calls.contains(right_id);
            let right_calls_left = right_usage.calls.contains(left_id);
            if shares_field || left_calls_right || right_calls_left {
                // Each unordered pair is visited once, so both insertions add a new edge.
                let _is_left_new = adjacency.entry(left).or_default().insert(right);
                let _is_right_new = adjacency.entry(right).or_default().insert(left);
            }
        }
    }
    adjacency
}

/// Return connected components from an undirected adjacency list.
fn connected_components(
    method_count: usize,
    adjacency: &HashMap<usize, HashSet<usize>>,
) -> Vec<Vec<usize>> {
    let mut components = Vec::new();
    let mut visited = HashSet::new();
    // Start one depth-first traversal for every method not assigned to an earlier component.
    for start in 0..method_count {
        if !visited.insert(start) {
            continue;
        }
        let mut pending = vec![start];
        let mut component = Vec::new();
        // Consume the reachable method set without relying on graph iteration order.
        while let Some(current) = pending.pop() {
            // Record the current method before expanding its unvisited neighbors.
            component.push(current);
            let Some(neighbors) = adjacency.get(&current) else {
                continue;
            };
            // Keep the pending stack bounded to neighbors discovered in this component.
            neighbors
                .iter()
                .copied()
                .filter(|&neighbor| visited.insert(neighbor))
                .for_each(|neighbor| pending.push(neighbor));
        }
        // Preserve each complete component for the threshold calculation.
        components.push(component);
    }
    components
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
