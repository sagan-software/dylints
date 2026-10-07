#![feature(rustc_private)]

//! Shared source metric calculations for maintainability lints.
//!
//! The module computes bounded control-flow, ABC, exit, and dependency metrics
//! directly from source-authored HIR. Callers use these values to keep individual
//! lints focused on policy thresholds while preserving macro and desugaring rules.

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
};

use dylint_linting as _;
use rustc_hir::{
    BinOpKind, Block, Body, Expr, ExprKind, HirId, LoopSource, MatchSource, Path, Stmt, StmtKind,
    UnOp,
    intravisit::{self, Visitor},
};
use rustc_middle::ty::TyCtxt;
use rustc_span::{
    DesugaringKind, ExpnKind, MacroKind, Span,
    def_id::{CRATE_DEF_ID, DefId, LocalDefId},
};

/// Calculate Cyclomatic Complexity for one source-authored callable.
///
/// The result counts independent source decisions and includes the callable's
/// initial execution path. Nested functions and macro-generated nodes add nothing,
/// while expressions written as macro arguments count like other source code.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # use rustc_hir::Body;
/// # let body: &Body<'_> = unimplemented!();
/// let _score = maintainability_support::cyclomatic_complexity(body);
/// ```
#[must_use]
pub fn cyclomatic_complexity(body: &Body<'_>) -> u32 {
    let mut counter = DecisionCounter::default();
    counter.visit_expr(body.value);
    counter.decision_count.saturating_add(1)
}

/// Calculate source Cognitive Complexity for one source-authored callable.
///
/// The score increases for structural breaks and their nesting depth, matching
/// the source-level profile used by the maintainability lint category.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # use rustc_hir::Body;
/// # let body: &Body<'_> = unimplemented!();
/// let _score = maintainability_support::cognitive_complexity(body);
/// ```
#[must_use]
pub fn cognitive_complexity(body: &Body<'_>) -> u32 {
    let mut counter = CognitiveCounter::default();
    counter.visit_expr(body.value);
    counter.score
}

/// Calculate a bounded `NPath` estimate for one source-authored callable.
///
/// Child routes are multiplied with saturation, so a large source expression
/// remains deterministic instead of overflowing the metric's integer type.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # use rustc_hir::Body;
/// # let body: &Body<'_> = unimplemented!();
/// let _score = maintainability_support::npath_complexity(body);
/// ```
#[must_use]
pub fn npath_complexity(body: &Body<'_>) -> u128 {
    npath_expression(body.value)
}

/// Calculate the Rust ABC vector for one source-authored callable.
///
/// Assignments, calls, and source control-flow conditions remain separate so the
/// caller can inspect both the vector components and its Euclidean magnitude.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # use rustc_hir::Body;
/// # let body: &Body<'_> = unimplemented!();
/// let _score = maintainability_support::abc_size(body);
/// ```
#[must_use]
pub fn abc_size(body: &Body<'_>) -> AbcSize {
    let mut counter = AbcCounter::default();
    counter.visit_expr(body.value);
    counter.size
}

/// Count explicit `return` expressions and desugared Rust `?` exits.
///
/// Macro-generated exits are excluded because they do not provide an actionable
/// source location in the inspected callable. Exits written as macro arguments count.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # use rustc_hir::Body;
/// # let body: &Body<'_> = unimplemented!();
/// let _exits = maintainability_support::exit_point_count(body);
/// ```
#[must_use]
pub fn exit_point_count(body: &Body<'_>) -> u32 {
    let mut counter = ExitPointCounter::default();
    counter.visit_expr(body.value);
    counter.count
}

/// Assignment, call, and condition counts for one callable.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbcSize {
    /// Number of source assignments and initialized local bindings counted in the callable.
    assignments: u32,
    /// Number of source function and method calls counted in the callable body.
    calls: u32,
    /// Number of source control-flow conditions contributing to the ABC vector.
    conditions: u32,
}

impl AbcSize {
    /// Return the assignment count collected from source-authored statements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use maintainability_support::AbcSize;
    /// let _count = AbcSize::default().assignments();
    /// ```
    #[must_use]
    pub const fn assignments(self) -> u32 {
        self.assignments
    }

    /// Return the call count collected from source-authored expressions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use maintainability_support::AbcSize;
    /// let _count = AbcSize::default().calls();
    /// ```
    #[must_use]
    pub const fn calls(self) -> u32 {
        self.calls
    }

    /// Return the condition count collected from source control-flow expressions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use maintainability_support::AbcSize;
    /// let _count = AbcSize::default().conditions();
    /// ```
    #[must_use]
    pub const fn conditions(self) -> u32 {
        self.conditions
    }

    /// Return the Euclidean magnitude of the three-component ABC vector.
    ///
    /// The magnitude combines assignments, calls, and conditions without exposing
    /// the internal storage fields used by the metric visitor.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use maintainability_support::AbcSize;
    /// let _magnitude = AbcSize::default().magnitude();
    /// ```
    #[must_use]
    pub fn magnitude(self) -> f64 {
        // Use fused multiply-adds to preserve a direct and bounded vector calculation.
        let assignments = f64::from(self.assignments);
        let calls = f64::from(self.calls);
        let conditions = f64::from(self.conditions);
        assignments
            .mul_add(assignments, calls.mul_add(calls, conditions * conditions))
            .sqrt()
    }
}

/// Return whether a span was generated by a bang, attribute, or derive macro.
///
/// The expansion chain is followed outward until the source call site, allowing
/// callers to exclude generated nodes from source-authored metric calculations.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// # extern crate rustc_span;
/// # use maintainability_support::is_macro_expansion;
/// # use rustc_span::DUMMY_SP;
/// let _generated = is_macro_expansion(DUMMY_SP);
/// ```
#[must_use]
pub fn is_macro_expansion(span: Span) -> bool {
    if !span.from_expansion() {
        return false;
    }

    let mut expansion = span.ctxt().outer_expn_data();
    loop {
        if matches!(
            expansion.kind,
            ExpnKind::Macro(MacroKind::Bang | MacroKind::Attr | MacroKind::Derive, _)
        ) {
            return true;
        }
        if !expansion.call_site.from_expansion() {
            return false;
        }
        expansion = expansion.call_site.ctxt().outer_expn_data();
    }
}

/// Record one local cross-module path edge when the path is source-authored.
///
/// Macro expansions, external definitions, and same-module references are ignored
/// so the resulting graph represents only actionable local module dependencies.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # extern crate rustc_middle;
/// # use std::collections::HashSet;
/// # use maintainability_support::record_path_edge;
/// # use rustc_hir::{HirId, Path};
/// # use rustc_middle::ty::TyCtxt;
/// # let (tcx, path, hir_id): (TyCtxt<'_>, &Path<'_>, HirId) = unimplemented!();
/// let mut edges = HashSet::new();
/// record_path_edge(tcx, path, hir_id, &mut edges);
/// ```
pub fn record_path_edge<S: BuildHasher>(
    tcx: TyCtxt<'_>,
    path: &Path<'_>,
    hir_id: HirId,
    edges: &mut HashSet<(LocalDefId, LocalDefId), S>,
) {
    if is_macro_expansion(path.span) {
        return;
    }
    let Some(target) = path.res.opt_def_id().and_then(DefId::as_local) else {
        return;
    };
    record_edge(
        top_level_module(tcx, hir_id.owner.def_id),
        top_level_module(tcx, target),
        edges,
    );
}

/// Record one resolved local definition edge when the expression is source-authored.
///
/// The target is reduced to a local definition before module identities are added,
/// preventing external or generated references from entering the dependency graph.
///
/// # Examples
///
/// ```rust,no_run
/// # #![feature(rustc_private)]
/// # extern crate rustc_hir;
/// # extern crate rustc_middle;
/// # extern crate rustc_span;
/// # use std::collections::HashSet;
/// # use maintainability_support::record_definition_edge;
/// # use rustc_hir::{HirId, def_id::DefId};
/// # use rustc_middle::ty::TyCtxt;
/// # use rustc_span::DUMMY_SP;
/// # let (tcx, hir_id, target): (TyCtxt<'_>, HirId, DefId) = unimplemented!();
/// let mut edges = HashSet::new();
/// record_definition_edge(tcx, hir_id, DUMMY_SP, target, &mut edges);
/// ```
pub fn record_definition_edge<S: BuildHasher>(
    tcx: TyCtxt<'_>,
    hir_id: HirId,
    span: Span,
    target: DefId,
    edges: &mut HashSet<(LocalDefId, LocalDefId), S>,
) {
    if is_macro_expansion(span) {
        return;
    }
    let Some(target) = target.as_local() else {
        return;
    };
    record_edge(
        top_level_module(tcx, hir_id.owner.def_id),
        top_level_module(tcx, target),
        edges,
    );
}

/// Group outgoing graph edges by their source module.
///
/// Each target is stored once per source, so repeated references do not inflate
/// fan-out measurements or change the graph's dependency semantics.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// # use std::collections::HashSet;
/// # use maintainability_support::fan_out_by_module;
/// let edges = HashSet::new();
/// let _fan_out = fan_out_by_module(&edges);
/// ```
#[must_use]
pub fn fan_out_by_module<S: BuildHasher>(
    edges: &HashSet<(LocalDefId, LocalDefId), S>,
) -> HashMap<LocalDefId, HashSet<LocalDefId>> {
    let mut fan_out = HashMap::new();
    for &(source, target) in edges {
        let _ = fan_out
            .entry(source)
            .or_insert_with(HashSet::new)
            .insert(target);
    }
    fan_out
}

/// Find strongly connected local-module components with at least two modules.
///
/// Components are returned only when a cycle contains multiple module vertices;
/// self references and disconnected modules do not produce cycle diagnostics.
/// The crate root is not a vertex: it is the composition root that declares and
/// re-exports every module, so edges to and from it cannot form an actionable cycle.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// # use std::collections::HashSet;
/// # use maintainability_support::dependency_cycles;
/// let edges = HashSet::new();
/// let _cycles = dependency_cycles(&edges);
/// ```
#[must_use]
pub fn dependency_cycles<S: BuildHasher>(
    edges: &HashSet<(LocalDefId, LocalDefId), S>,
) -> Vec<HashSet<LocalDefId>> {
    // Drop crate-root edges before building the graph so the root cannot join a component.
    let module_edges: HashSet<_> = edges
        .iter()
        .copied()
        .filter(|&(source, target)| source != CRATE_DEF_ID && target != CRATE_DEF_ID)
        .collect();
    let graph = fan_out_by_module(&module_edges);
    let mut vertices = HashSet::new();
    for &(source, target) in &module_edges {
        let _ = vertices.insert(source);
        let _ = vertices.insert(target);
    }

    let mut cycles = Vec::new();
    let mut assigned = HashSet::new();
    for &start in &vertices {
        if assigned.contains(&start) {
            continue;
        }
        let forward = reachable(start, &graph);
        let component: HashSet<_> = forward
            .into_iter()
            .filter(|candidate| reachable(*candidate, &graph).contains(&start))
            .collect();
        assigned.extend(component.iter().copied());
        if component.len() > 1 {
            cycles.push(component);
        }
    }
    cycles
}

/// Counts source decisions without entering nested callable bodies.
#[derive(Default)]
struct DecisionCounter {
    /// Number of independent source decisions encountered in the current body.
    decision_count: u32,
}

/// Counts the three Rust ABC components without entering nested callables.
#[derive(Default)]
struct AbcCounter {
    /// Accumulated source metrics.
    size: AbcSize,
}

impl AbcCounter {
    /// Add one bounded assignment.
    const fn add_assignment(&mut self) {
        self.size.assignments = self.size.assignments.saturating_add(1);
    }

    /// Add one bounded call.
    const fn add_call(&mut self) {
        self.size.calls = self.size.calls.saturating_add(1);
    }

    /// Add a bounded number of conditions.
    const fn add_conditions(&mut self, conditions: u32) {
        self.size.conditions = self.size.conditions.saturating_add(conditions);
    }
}

impl<'tcx> Visitor<'tcx> for AbcCounter {
    /// Count initialized bindings before walking their initializer.
    fn visit_stmt(&mut self, statement: &'tcx Stmt<'tcx>) {
        // Treat one initialized binding as one assignment regardless of pattern width.
        if !is_macro_expansion(statement.span)
            && matches!(statement.kind, StmtKind::Let(local) if local.init.is_some())
        {
            self.add_assignment();
        }
        intravisit::walk_stmt(self, statement);
    }

    /// Classify source expressions, then continue through every child.
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        // Generated and desugared nodes do not count, but source-authored children, such as
        // macro arguments, still do.
        if is_source_expression(expression) {
            if matches!(
                expression.kind,
                ExprKind::Assign(..) | ExprKind::AssignOp(..)
            ) {
                self.add_assignment();
            } else if matches!(
                expression.kind,
                ExprKind::Call(..) | ExprKind::MethodCall(..)
            ) {
                self.add_call();
            } else {
                self.add_conditions(abc_expression_conditions(expression));
            }
        }
        intravisit::walk_expr(self, expression);
    }
}

/// Return whether an expression is source-authored and not compiler-desugared.
fn is_source_expression(expression: &Expr<'_>) -> bool {
    !is_macro_expansion(expression.span) && expression.span.desugaring_kind().is_none()
}

/// Count source control-flow tests under the Rust ABC profile.
fn abc_expression_conditions(expression: &Expr<'_>) -> u32 {
    // Count source control-flow alternatives rather than every comparison expression.
    if matches!(
        expression.kind,
        ExprKind::If(..) | ExprKind::Loop(_, _, LoopSource::While | LoopSource::ForLoop, _)
    ) {
        return 1;
    }
    if let ExprKind::Match(_, arms, MatchSource::Normal | MatchSource::Postfix) = expression.kind {
        let alternatives = arms.len().saturating_sub(1);
        let guards = arms.iter().filter(|arm| arm.guard.is_some()).count();
        return u32::try_from(alternatives.saturating_add(guards)).unwrap_or(u32::MAX);
    }
    u32::from(matches!(
        expression.kind,
        ExprKind::Binary(operation, _, _)
            if matches!(operation.node, BinOpKind::And | BinOpKind::Or)
    ))
}

/// Counts explicit and `?`-desugared exits without entering nested callables.
#[derive(Default)]
struct ExitPointCounter {
    /// Number of exits encountered in the current body.
    count: u32,
}

impl<'tcx> Visitor<'tcx> for ExitPointCounter {
    /// Count one source exit and continue through its operands.
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        // Macro exits have no actionable source site, but macro arguments are source code.
        // Count explicit returns and one outer match for each `?` desugaring.
        if !is_macro_expansion(expression.span)
            && (matches!(expression.kind, ExprKind::Ret(..))
                && expression.span.desugaring_kind().is_none()
                || matches!(
                    expression.kind,
                    ExprKind::Match(_, _, MatchSource::TryDesugar(_))
                ))
        {
            self.count = self.count.saturating_add(1);
        }
        intravisit::walk_expr(self, expression);
    }
}

impl DecisionCounter {
    /// Add a bounded number of decision points.
    const fn add(&mut self, decisions: u32) {
        self.decision_count = self.decision_count.saturating_add(decisions);
    }
}

/// Return the decision contribution made by one source expression.
fn expression_decisions(expr: &Expr<'_>) -> u32 {
    // Ignore compiler-generated conditionals before classifying source decisions.
    if matches!(expr.kind, ExprKind::If(..)) && expr.span.desugaring_kind().is_some() {
        return 0;
    }
    // Count source branches, loops, matches, and short-circuit operators.
    if matches!(
        expr.kind,
        ExprKind::If(..)
            | ExprKind::Loop(_, _, LoopSource::While | LoopSource::ForLoop, _)
            | ExprKind::Match(_, _, MatchSource::TryDesugar(_))
    ) {
        return 1;
    }
    if let ExprKind::Match(_, arms, MatchSource::Normal | MatchSource::Postfix) = expr.kind {
        let alternatives = arms.len().saturating_sub(1);
        let guards = arms.iter().filter(|arm| arm.guard.is_some()).count();
        return u32::try_from(alternatives.saturating_add(guards)).unwrap_or(u32::MAX);
    }
    if let ExprKind::Binary(operation, _, _) = expr.kind
        && matches!(operation.node, BinOpKind::And | BinOpKind::Or)
    {
        return 1;
    }
    0
}

impl<'tcx> Visitor<'tcx> for DecisionCounter {
    /// Count decision-bearing expressions, then continue through every child.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Generated nodes add nothing, but source-authored macro arguments below them still count.
        if !is_macro_expansion(expr.span) {
            self.add(expression_decisions(expr));
        }
        intravisit::walk_expr(self, expr);
    }
}

/// Counts source control-flow breaks and their nesting burden.
#[derive(Default)]
struct CognitiveCounter {
    /// Current control-flow nesting depth.
    nesting: u32,
    /// Accumulated bounded score.
    score: u32,
}

impl CognitiveCounter {
    /// Add one structural break and its current nesting penalty.
    const fn add_nested_break(&mut self) {
        self.score = self
            .score
            .saturating_add(1_u32.saturating_add(self.nesting));
    }

    /// Visit one expression while one level deeper in control flow.
    fn visit_nested<'tcx>(&mut self, expr: &'tcx Expr<'tcx>) {
        self.nesting = self.nesting.saturating_add(1);
        self.visit_expr(expr);
        self.nesting = self.nesting.saturating_sub(1);
    }

    /// Visit one source `if`, including its condition and optional continuation.
    fn visit_if_expression<'tcx>(
        &mut self,
        condition: &'tcx Expr<'tcx>,
        then_expression: &'tcx Expr<'tcx>,
        else_expression: Option<&'tcx Expr<'tcx>>,
    ) {
        self.add_nested_break();
        self.visit_expr(condition);
        self.visit_nested(then_expression);
        if let Some(else_expression) = else_expression {
            if matches!(else_expression.kind, ExprKind::If(..)) {
                self.visit_expr(else_expression);
            } else {
                self.visit_nested(else_expression);
            }
        }
    }

    /// Visit a control-flow structure whose children all add one nesting level.
    fn visit_nested_structure<'tcx>(&mut self, expr: &'tcx Expr<'tcx>) {
        self.add_nested_break();
        self.nesting = self.nesting.saturating_add(1);
        intravisit::walk_expr(self, expr);
        self.nesting = self.nesting.saturating_sub(1);
    }
}

impl<'tcx> Visitor<'tcx> for CognitiveCounter {
    /// Count structural breaks while preserving the source nesting relationship.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Generated and desugared structures, including the loop behind each `.await`, add
        // nothing; source-authored children such as macro arguments are still visited.
        if is_macro_expansion(expr.span) || is_desugared_if_or_await(expr) {
            intravisit::walk_expr(self, expr);
        } else if let ExprKind::If(condition, then_expression, else_expression) = expr.kind {
            self.visit_if_expression(condition, then_expression, else_expression);
        } else if matches!(
            expr.kind,
            ExprKind::Loop(..) | ExprKind::Match(_, _, MatchSource::Normal | MatchSource::Postfix)
        ) {
            self.visit_nested_structure(expr);
        } else if let ExprKind::Binary(operation, _, _) = expr.kind
            && matches!(operation.node, BinOpKind::And | BinOpKind::Or)
        {
            self.score = self.score.saturating_add(1);
            intravisit::walk_expr(self, expr);
        } else {
            intravisit::walk_expr(self, expr);
        }
    }
}

/// Return whether an expression is a desugared `if` or the generated loop of an `.await`.
fn is_desugared_if_or_await(expr: &Expr<'_>) -> bool {
    if let ExprKind::If(..) = expr.kind {
        return expr.span.desugaring_kind().is_some();
    }

    if let ExprKind::Loop(..) = expr.kind {
        return expr.span.desugaring_kind() == Some(DesugaringKind::Await);
    }

    false
}

/// Multiplies complete child-expression route counts for a containing expression.
struct ChildPathProduct {
    /// Accumulated child path count, saturated to keep diagnostics deterministic.
    paths: u128,
}

impl<'tcx> Visitor<'tcx> for ChildPathProduct {
    /// Multiply by one complete child expression without walking it twice.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        self.paths = self.paths.saturating_mul(npath_expression(expr));
    }
}

/// Calculate one expression's acyclic route count.
fn npath_expression(expr: &Expr<'_>) -> u128 {
    // Generated structures, including the loop behind each `.await`, add no routes of their
    // own, while source-authored children such as macro arguments still multiply.
    if is_macro_expansion(expr.span) || is_desugared_if_or_await(expr) {
        return child_path_product(expr);
    }

    if let ExprKind::If(condition, then_expression, else_expression) = expr.kind {
        let outcomes = condition_outcomes(condition);
        let then_paths = npath_expression(then_expression);
        let else_paths = else_expression.map_or(1, npath_expression);
        return outcomes
            .true_paths
            .saturating_mul(then_paths)
            .saturating_add(outcomes.false_paths.saturating_mul(else_paths));
    }
    if let ExprKind::Loop(block, ..) = expr.kind {
        return block_path_product(block).saturating_add(1);
    }
    if let ExprKind::Match(scrutinee, arms, MatchSource::Normal | MatchSource::Postfix) = expr.kind
    {
        let alternatives = arms.iter().fold(0_u128, |paths, arm| {
            let body_paths = npath_expression(arm.body);
            let guarded_paths = if arm.guard.is_some() {
                body_paths.saturating_add(1)
            } else {
                body_paths
            };
            paths.saturating_add(guarded_paths)
        });
        return npath_expression(scrutinee).saturating_mul(alternatives.max(1));
    }
    if let ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) = expr.kind {
        return npath_expression(scrutinee).saturating_mul(2);
    }
    if let ExprKind::Binary(operation, _, _) = expr.kind
        && matches!(operation.node, BinOpKind::And | BinOpKind::Or)
    {
        return condition_outcomes(expr).total();
    }
    child_path_product(expr)
}

/// Multiply the complete route counts of an expression's direct children.
fn child_path_product(expr: &Expr<'_>) -> u128 {
    let mut product = ChildPathProduct { paths: 1 };
    intravisit::walk_expr(&mut product, expr);
    product.paths
}

/// Multiply the complete route counts of a block's direct child expressions.
fn block_path_product(block: &Block<'_>) -> u128 {
    let mut product = ChildPathProduct { paths: 1 };
    intravisit::walk_block(&mut product, block);
    product.paths
}

/// True and false evaluation routes through one Boolean condition.
#[derive(Clone, Copy)]
struct ConditionOutcomes {
    /// Routes that finish with a true result.
    true_paths: u128,
    /// Routes that finish with a false result.
    false_paths: u128,
}

impl ConditionOutcomes {
    /// Return every evaluation route regardless of the final result.
    const fn total(self) -> u128 {
        self.true_paths.saturating_add(self.false_paths)
    }
}

/// Count short-circuit evaluation routes by their Boolean result.
fn condition_outcomes(expr: &Expr<'_>) -> ConditionOutcomes {
    // Generated expressions represent an unconstrained Boolean branch pair.
    if is_macro_expansion(expr.span) {
        return ConditionOutcomes {
            true_paths: 1,
            false_paths: 1,
        };
    }

    if let ExprKind::Binary(operation, left, right) = expr.kind
        && operation.node == BinOpKind::And
    {
        let left = condition_outcomes(left);
        let right = condition_outcomes(right);
        return ConditionOutcomes {
            true_paths: left.true_paths.saturating_mul(right.true_paths),
            false_paths: left
                .false_paths
                .saturating_add(left.true_paths.saturating_mul(right.false_paths)),
        };
    }
    if let ExprKind::Binary(operation, left, right) = expr.kind
        && operation.node == BinOpKind::Or
    {
        let left = condition_outcomes(left);
        let right = condition_outcomes(right);
        return ConditionOutcomes {
            true_paths: left
                .true_paths
                .saturating_add(left.false_paths.saturating_mul(right.true_paths)),
            false_paths: left.false_paths.saturating_mul(right.false_paths),
        };
    }
    if let ExprKind::Unary(UnOp::Not, operand) = expr.kind {
        let operand = condition_outcomes(operand);
        return ConditionOutcomes {
            true_paths: operand.false_paths,
            false_paths: operand.true_paths,
        };
    }
    ConditionOutcomes {
        true_paths: 1,
        false_paths: 1,
    }
}

/// Collapse any local definition to its direct child module below the crate root.
///
/// Definitions directly in the crate root, other than modules, collapse to the root.
fn top_level_module(tcx: TyCtxt<'_>, mut def_id: LocalDefId) -> LocalDefId {
    // Climb until the parent is the crate root, then keep only module children.
    while def_id != CRATE_DEF_ID {
        let parent = tcx.local_parent(def_id);
        if parent == CRATE_DEF_ID {
            return if tcx.def_kind(def_id) == rustc_hir::def::DefKind::Mod {
                def_id
            } else {
                CRATE_DEF_ID
            };
        }
        def_id = parent;
    }
    CRATE_DEF_ID
}

/// Insert one edge when it crosses distinct top-level local modules.
fn record_edge<S: BuildHasher>(
    source: LocalDefId,
    target: LocalDefId,
    edges: &mut HashSet<(LocalDefId, LocalDefId), S>,
) {
    if source != target {
        let _ = edges.insert((source, target));
    }
}

/// Return every module reachable from one graph vertex, including itself.
fn reachable(
    start: LocalDefId,
    graph: &HashMap<LocalDefId, HashSet<LocalDefId>>,
) -> HashSet<LocalDefId> {
    let mut found = HashSet::from([start]);
    let mut pending = vec![start];
    while let Some(source) = pending.pop() {
        if let Some(targets) = graph.get(&source) {
            for &target in targets {
                if found.insert(target) {
                    pending.push(target);
                }
            }
        }
    }
    found
}
