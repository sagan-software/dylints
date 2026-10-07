//! Track direct entity origins from one Bevy query parameter.

use rustc_hir::{Body, Expr, ExprKind, PatKind, def::Res, intravisit::Visitor};
use rustc_lint::LateContext;

use super::helpers::{bevy_ecs_method_name, closure_body, local_path_id};
use super::{expression_has_type, type_is_named};

/// Return whether a body performs a fixed typed access through an entity view.
pub(crate) fn body_contains_fixed_entity_access<'tcx>(
    cx: &LateContext<'tcx>,
    body: &Body<'tcx>,
    query_binding: rustc_hir::HirId,
) -> bool {
    let mut visitor = FixedEntityAccessVisitor {
        cx,
        query_binding,
        entity_bindings: Vec::new(),
        is_found: false,
    };
    visitor.visit_expr(body.value);
    visitor.is_found
}

/// Visitor that links fixed typed entity access to one query parameter.
pub(crate) struct FixedEntityAccessVisitor<'a, 'tcx> {
    /// Lint context used to resolve receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// Query parameter whose yielded entities count for this scan.
    pub(crate) query_binding: rustc_hir::HirId,
    /// Entity bindings yielded by direct iteration of the selected query.
    pub(crate) entity_bindings: Vec<rustc_hir::HirId>,
    /// Whether a supported access uses one of the selected query's entities.
    pub(crate) is_found: bool,
}

impl<'tcx> FixedEntityAccessVisitor<'_, 'tcx> {
    /// Remove query provenance after an entity binding receives an unknown source.
    fn forget_unknown_assignment(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Assign(target, value, _) = expr.kind
            && let Some(binding) = local_binding_path(self.cx, target)
            && self.entity_bindings.contains(&binding)
            && !query_entity_source(self.cx, value, self.query_binding, &self.entity_bindings)
        {
            self.entity_bindings
                .retain(|entity_binding| *entity_binding != binding);
        }
    }

    /// Link bindings tested by an `if let` condition sourced from the selected query.
    fn link_if_let_bindings(&mut self, expr: &'tcx Expr<'tcx>) {
        let ExprKind::If(condition, _, _) = expr.kind else {
            return;
        };
        let ExprKind::Let(let_expr) = condition.kind else {
            return;
        };
        if query_entity_source(
            self.cx,
            let_expr.init,
            self.query_binding,
            &self.entity_bindings,
        ) {
            self.entity_bindings
                .extend(entity_proxy_pattern_bindings(self.cx, let_expr.pat));
        }
    }

    /// Link match-arm bindings when the scrutinee is sourced from the selected query.
    fn link_match_bindings(&mut self, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Match(source, arms, _) = expr.kind else {
            return;
        };
        if query_entity_source(self.cx, source, self.query_binding, &self.entity_bindings) {
            for arm in arms {
                self.entity_bindings
                    .extend(entity_proxy_pattern_bindings(self.cx, arm.pat));
            }
        }
    }

    /// Link `for`-loop item bindings when the iterator starts at the selected query.
    fn link_for_loop_bindings(&mut self, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Match(source, arms, rustc_hir::MatchSource::ForLoopDesugar) = expr.kind
        else {
            return;
        };
        let Some(iterator_arm) = arms.first() else {
            return;
        };
        let PatKind::Binding(_, iterator_binding, _, None) = iterator_arm.pat.kind else {
            return;
        };
        if for_loop_uses_query(self.cx, source, self.query_binding) {
            let mut item_visitor = ForLoopEntityBindingVisitor {
                cx: self.cx,
                iterator_binding,
                entity_bindings: Vec::new(),
            };
            item_visitor.visit_expr(iterator_arm.body);
            self.entity_bindings.extend(item_visitor.entity_bindings);
        }
    }

    /// Link the closure argument of a direct query iterator's `for_each` call.
    fn link_for_each_bindings(&mut self, expr: &'tcx Expr<'tcx>) {
        self.entity_bindings.extend(query_for_each_entity_bindings(
            self.cx,
            expr,
            self.query_binding,
        ));
    }

    /// Record a supported fixed typed access through an entity from this query.
    fn visit_fixed_typed_access(&mut self, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(_, receiver, _, _) = expr.kind else {
            return;
        };
        let has_entity_proxy = expression_has_type(self.cx, receiver, "bevy_ecs", "EntityRef")
            || expression_has_type(self.cx, receiver, "bevy_ecs", "EntityMut");
        let has_query_origin =
            query_entity_source(self.cx, receiver, self.query_binding, &self.entity_bindings);
        let is_typed_access = bevy_ecs_method_name(self.cx, expr).is_some_and(|name| {
            matches!(
                name.as_str(),
                "contains"
                    | "get"
                    | "get_components"
                    | "get_components_mut"
                    | "get_mut"
                    | "get_ref"
            )
        });
        if has_entity_proxy && has_query_origin && is_typed_access {
            self.is_found = true;
        }
    }
}

impl<'tcx> Visitor<'tcx> for FixedEntityAccessVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_stmt(&mut self, statement: &'tcx rustc_hir::Stmt<'tcx>) {
        if let rustc_hir::StmtKind::Let(binding) = statement.kind
            && let Some(initializer) = binding.init
            && query_entity_source(
                self.cx,
                initializer,
                self.query_binding,
                &self.entity_bindings,
            )
        {
            // Preserve query origin when a direct result is first stored in a local.
            self.entity_bindings
                .extend(entity_proxy_pattern_bindings(self.cx, binding.pat));
        }
        rustc_hir::intravisit::walk_stmt(self, statement);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Link bindings before walking closures, match arms, and loop bodies.
        self.link_if_let_bindings(expr);
        self.link_match_bindings(expr);
        self.link_for_loop_bindings(expr);
        self.link_for_each_bindings(expr);
        self.visit_fixed_typed_access(expr);
        rustc_hir::intravisit::walk_expr(self, expr);

        // Keep the previous origin while the assignment right-hand side runs.
        self.forget_unknown_assignment(expr);
    }
}

/// Return whether a `for` loop's `IntoIterator` input comes from one query binding.
fn for_loop_uses_query(
    cx: &LateContext<'_>,
    source: &Expr<'_>,
    query_binding: rustc_hir::HirId,
) -> bool {
    let ExprKind::Call(callee, arguments) = source.kind else {
        return false;
    };
    let [iterable] = arguments else {
        return false;
    };
    resolved_trait_method_named(cx, callee, "IntoIterator", "into_iter")
        && query_iterator_source(cx, iterable, query_binding)
}

/// Return whether an expression is a direct query iterator from one parameter.
fn query_iterator_source(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    query_binding: rustc_hir::HirId,
) -> bool {
    if let ExprKind::AddrOf(_, _, inner)
    | ExprKind::DropTemps(inner)
    | ExprKind::Cast(inner, _)
    | ExprKind::Type(inner, _) = expr.kind
    {
        return query_iterator_source(cx, inner, query_binding);
    }
    if let ExprKind::Path(_) = expr.kind {
        return local_path_id(cx, expr) == Some(query_binding);
    }
    if let ExprKind::MethodCall(_, receiver, [], _) = expr.kind {
        return bevy_ecs_method_name(cx, expr)
            .is_some_and(|name| matches!(name.as_str(), "iter" | "iter_mut"))
            && expression_has_type(cx, receiver, "bevy_ecs", "Query")
            && local_path_id(cx, receiver) == Some(query_binding);
    }
    false
}

/// Return whether an entity value comes from one query through a supported path.
fn query_entity_source(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    query_binding: rustc_hir::HirId,
    entity_bindings: &[rustc_hir::HirId],
) -> bool {
    if let ExprKind::AddrOf(_, _, inner)
    | ExprKind::DropTemps(inner)
    | ExprKind::Cast(inner, _)
    | ExprKind::Type(inner, _) = expr.kind
    {
        return query_entity_source(cx, inner, query_binding, entity_bindings);
    }
    if let ExprKind::Path(_) = expr.kind {
        return local_binding_path(cx, expr)
            .is_some_and(|binding| entity_bindings.contains(&binding));
    }
    if let ExprKind::MethodCall(_, receiver, _, _) = expr.kind {
        let is_query_access = bevy_ecs_method_name(cx, expr).is_some_and(|name| {
            matches!(name.as_str(), "get" | "get_mut" | "single" | "single_mut")
        }) && expression_has_type(cx, receiver, "bevy_ecs", "Query")
            && local_binding_path(cx, receiver) == Some(query_binding);
        let is_iterator_next = cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .is_some_and(|def_id| {
                cx.tcx.item_name(def_id).as_str() == "next"
                    && cx.tcx.trait_of_assoc(def_id).is_some_and(|trait_def_id| {
                        cx.tcx
                            .is_diagnostic_item(rustc_span::sym::Iterator, trait_def_id)
                    })
            })
            && query_iterator_source(cx, receiver, query_binding);
        return is_query_access
            || is_iterator_next
            || result_extraction_method(cx, expr, receiver)
                && query_entity_source(cx, receiver, query_binding, entity_bindings);
    }
    false
}

/// Return whether an exact `Result` or `Option` extraction method removes its wrapper.
fn result_extraction_method(cx: &LateContext<'_>, expr: &Expr<'_>, receiver: &Expr<'_>) -> bool {
    let is_extraction = cx
        .typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|def_id| matches!(cx.tcx.item_name(def_id).as_str(), "expect" | "unwrap"));
    is_extraction
        && (type_is_named(
            cx,
            cx.typeck_results().expr_ty_adjusted(receiver),
            "core",
            "Option",
        ) || type_is_named(
            cx,
            cx.typeck_results().expr_ty_adjusted(receiver),
            "core",
            "Result",
        ))
}

/// Return whether a resolved path is the named method of a core iterator trait.
fn resolved_trait_method_named(
    cx: &LateContext<'_>,
    callee: &Expr<'_>,
    trait_name: &str,
    method_name: &str,
) -> bool {
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return false;
    };
    cx.tcx.item_name(def_id).as_str() == method_name
        && cx.tcx.trait_of_assoc(def_id).is_some_and(|trait_def_id| {
            cx.tcx.crate_name(trait_def_id.krate).as_str() == "core"
                && cx.tcx.item_name(trait_def_id).as_str() == trait_name
        })
}

/// Return the local binding beneath address and compiler wrapper expressions.
fn local_binding_path(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<rustc_hir::HirId> {
    if let ExprKind::AddrOf(_, _, inner)
    | ExprKind::DropTemps(inner)
    | ExprKind::Cast(inner, _)
    | ExprKind::Type(inner, _) = expr.kind
    {
        return local_binding_path(cx, inner);
    }
    if let ExprKind::Path(_) = expr.kind {
        return local_path_id(cx, expr);
    }
    None
}

/// Visitor that finds the item pattern for one desugared query `for` loop.
struct ForLoopEntityBindingVisitor<'a, 'tcx> {
    /// Lint context used to resolve the `Iterator::next` source and item types.
    cx: &'a LateContext<'tcx>,
    /// Generated iterator local introduced by the enclosing `for` loop.
    iterator_binding: rustc_hir::HirId,
    /// Entity proxy bindings yielded by the matching loop.
    entity_bindings: Vec<rustc_hir::HirId>,
}

impl<'tcx> Visitor<'tcx> for ForLoopEntityBindingVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Match(source, arms, rustc_hir::MatchSource::ForLoopDesugar) = expr.kind
            && iterator_next_uses_binding(self.cx, source, self.iterator_binding)
        {
            // Only item patterns bind the yielded values; the `None` pattern adds no bindings.
            for arm in arms {
                self.entity_bindings
                    .extend(entity_proxy_pattern_bindings(self.cx, arm.pat));
            }
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Return whether a desugared `Iterator::next` call reads one loop iterator.
fn iterator_next_uses_binding(
    cx: &LateContext<'_>,
    source: &Expr<'_>,
    iterator_binding: rustc_hir::HirId,
) -> bool {
    let ExprKind::Call(callee, arguments) = source.kind else {
        return false;
    };
    let [receiver] = arguments else {
        return false;
    };
    resolved_trait_method_named(cx, callee, "Iterator", "next")
        && local_binding_path(cx, receiver) == Some(iterator_binding)
}

/// Return entity proxy bindings nested in one query item pattern.
fn entity_proxy_pattern_bindings<'tcx>(
    cx: &LateContext<'tcx>,
    pattern: &'tcx rustc_hir::Pat<'tcx>,
) -> Vec<rustc_hir::HirId> {
    let mut visitor = EntityProxyPatternVisitor {
        cx,
        bindings: Vec::new(),
    };
    visitor.visit_pat(pattern);
    visitor.bindings
}

/// Visitor that collects `EntityRef` and `EntityMut` pattern bindings.
struct EntityProxyPatternVisitor<'a, 'tcx> {
    /// Lint context used to inspect each pattern's resolved type.
    cx: &'a LateContext<'tcx>,
    /// Binding IDs whose values have an entity proxy type.
    bindings: Vec<rustc_hir::HirId>,
}

impl<'tcx> Visitor<'tcx> for EntityProxyPatternVisitor<'_, 'tcx> {
    fn visit_pat(&mut self, pattern: &'tcx rustc_hir::Pat<'tcx>) {
        if let PatKind::Binding(_, binding, _, _) = pattern.kind {
            let pattern_type = self.cx.typeck_results().node_type(pattern.hir_id);
            if (type_is_named(self.cx, pattern_type, "bevy_ecs", "EntityRef")
                || type_is_named(self.cx, pattern_type, "bevy_ecs", "EntityMut"))
                && !self.bindings.contains(&binding)
            {
                self.bindings.push(binding);
            }
        }
        rustc_hir::intravisit::walk_pat(self, pattern);
    }
}

/// Return entity bindings from one `Iterator::for_each` closure over a query iterator.
fn query_for_each_entity_bindings(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    query_binding: rustc_hir::HirId,
) -> Vec<rustc_hir::HirId> {
    let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
        return Vec::new();
    };
    let is_iterator_for_each = cx
        .typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|def_id| {
            cx.tcx.item_name(def_id).as_str() == "for_each"
                && cx.tcx.trait_of_assoc(def_id).is_some_and(|trait_def_id| {
                    cx.tcx
                        .is_diagnostic_item(rustc_span::sym::Iterator, trait_def_id)
                })
        });
    if !is_iterator_for_each || !query_iterator_source(cx, receiver, query_binding) {
        return Vec::new();
    }

    // The resolved `Iterator::for_each` call has one closure argument.
    let Some(Expr {
        kind: ExprKind::Closure(closure),
        ..
    }) = arguments.first()
    else {
        return Vec::new();
    };
    let Some(closure_body) = closure_body(cx, closure.body) else {
        return Vec::new();
    };
    closure_body
        .params
        .first()
        .map(|parameter| entity_proxy_pattern_bindings(cx, parameter.pat))
        .unwrap_or_default()
}
