#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Detects repeated guard attachment to a Bevy `LoadBuilder` within one body.
//! The lint resolves `AssetServer::load_builder` and `LoadBuilder` methods,
//! then tracks guard counts through chains and direct local moves. It reports
//! when `with_guard` replaces a known earlier guard on that same builder.
//! Unknown mutable escapes and opaque closure calls invalidate affected facts.
//! The lint does not inspect guard implementations or infer unrelated builders.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use dylint_linting as _;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{
    Body, Expr, ExprKind, HirId, Mutability, PatKind, Stmt, StmtKind, def::Res, intravisit,
};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_middle::ty;
use rustc_span::Span;
use std::collections::HashMap;

#[cfg(test)]
use bevy_app::{App, TaskPoolPlugin};
#[cfg(test)]
use bevy_asset::{AssetServer, LoadBuilder};
#[cfg(test)]
use bevy_ecs as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_LOAD_BUILDER_REPLACED_GUARD,
    Warn,
    "checks for repeated Bevy asset load guards",
    BevyLoadBuilderReplacedGuard
}

impl<'tcx> LateLintPass<'tcx> for BevyLoadBuilderReplacedGuard {
    /// Check each function body for repeated guards on one resolved load builder.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _: Span,
        _: rustc_span::def_id::LocalDefId,
    ) {
        let mut visitor = LoadBuilderVisitor {
            cx,
            guards_by_binding: HashMap::new(),
        };
        visitor.visit_body(body);
    }
}

/// Tracks known guard counts while walking one function body in source order.
struct LoadBuilderVisitor<'a, 'tcx> {
    /// Rustc context for resolved Bevy methods and local bindings.
    cx: &'a LateContext<'tcx>,
    /// Guard counts for direct local moves from known load builders.
    guards_by_binding: HashMap<HirId, usize>,
}

impl<'tcx> Visitor<'tcx> for LoadBuilderVisitor<'_, 'tcx> {
    /// Record immutable local builder bindings after their initializers are visited.
    fn visit_stmt(&mut self, statement: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(local) = statement.kind {
            if let Some(initializer) = local.init {
                // Resolve nested builder chains before recording the new local binding.
                self.visit_expr(initializer);
                if let PatKind::Binding(_, binding_id, _, None) = local.pat.kind
                    && bevy_support::expression_has_type(
                        self.cx,
                        initializer,
                        "bevy_asset",
                        "LoadBuilder",
                    )
                    && let Some(guard_count) =
                        known_guard_count(self.cx, initializer, &self.guards_by_binding)
                {
                    let _previous_guard_count =
                        self.guards_by_binding.insert(binding_id, guard_count);
                }
            }
            // Visit a let-else block before leaving this statement to retain source order.
            if let Some(else_block) = local.els {
                self.visit_block(else_block);
            }
            return;
        }
        intravisit::walk_stmt(self, statement);
    }

    /// Visit method receivers before reporting the current call in a builder chain.
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        // Preserve evaluation order for tracked calls, assignments, and opaque calls.
        match expression.kind {
            ExprKind::MethodCall(_, receiver, arguments, _) => {
                self.visit_method_call(expression, receiver, arguments);
            }
            ExprKind::Assign(left, right, _) => self.visit_assignment(left, right),
            ExprKind::Call(callee, arguments) => self.visit_opaque_call(callee, arguments),
            // Traverse syntax outside the tracked forms without changing its semantics.
            ExprKind::ConstBlock(_)
            | ExprKind::Array(_)
            | ExprKind::Use(..)
            | ExprKind::Tup(_)
            | ExprKind::Binary(..)
            | ExprKind::Unary(..)
            | ExprKind::Lit(_)
            | ExprKind::Cast(..)
            | ExprKind::Type(..)
            | ExprKind::DropTemps(_)
            | ExprKind::Let(_)
            | ExprKind::If(..)
            | ExprKind::Loop(..)
            | ExprKind::Match(..)
            | ExprKind::Closure(_)
            | ExprKind::Block(..)
            | ExprKind::AssignOp(..)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::Path(_)
            | ExprKind::AddrOf(..)
            | ExprKind::Break(..)
            | ExprKind::Continue(_)
            | ExprKind::Ret(_)
            | ExprKind::Become(_)
            | ExprKind::InlineAsm(_)
            | ExprKind::OffsetOf(..)
            | ExprKind::Struct(..)
            | ExprKind::Repeat(..)
            | ExprKind::Yield(..)
            | ExprKind::UnsafeBinderCast(..)
            | ExprKind::Err(_) => intravisit::walk_expr(self, expression),
        }
    }

    /// Avoid attributing a builder captured by a closure to its enclosing function.
    fn visit_nested_body(&mut self, _: rustc_hir::BodyId) {}
}

impl<'tcx> LoadBuilderVisitor<'_, 'tcx> {
    /// Visit a method call before invalidating facts passed to untracked APIs.
    fn visit_method_call(
        &mut self,
        expression: &'tcx Expr<'tcx>,
        receiver: &'tcx Expr<'tcx>,
        arguments: &'tcx [Expr<'tcx>],
    ) {
        // Visit nested calls before applying the outer builder method.
        self.visit_expr(receiver);
        for argument in arguments {
            self.visit_expr(argument);
        }
        self.check_guard_call(expression, receiver);

        // Unknown methods may mutate borrowed builders or invoke captured closures.
        if !is_tracked_load_builder_method(self.cx, expression, receiver) {
            self.invalidate_escaped_builders(arguments);
            if is_closure_typed(self.cx, receiver) || has_closure_argument(self.cx, arguments) {
                self.guards_by_binding.clear();
            }
        }
    }

    /// Visit both assignment operands before discarding the destination's old facts.
    fn visit_assignment(&mut self, left: &'tcx Expr<'tcx>, right: &'tcx Expr<'tcx>) {
        // Preserve the source order before applying the identity change.
        self.visit_expr(left);
        self.visit_expr(right);
        self.forget_reassigned_builder(left);
    }

    /// Visit an opaque call and invalidate facts for closure invocation or escape.
    fn visit_opaque_call(&mut self, callee: &'tcx Expr<'tcx>, arguments: &'tcx [Expr<'tcx>]) {
        // Analyze arguments before the call can mutate them or execute a closure body.
        self.visit_expr(callee);
        for argument in arguments {
            self.visit_expr(argument);
        }
        // Opaque closure execution can replace any captured builder binding.
        if is_closure_typed(self.cx, callee) || has_closure_argument(self.cx, arguments) {
            self.guards_by_binding.clear();
        }
        self.invalidate_escaped_builders(arguments);
    }

    /// Forget guard counts for mutable builders passed to opaque calls.
    fn invalidate_escaped_builders(&mut self, arguments: &[Expr<'_>]) {
        // Restrict escape handling to mutable references of Bevy's exact builder type.
        for argument in arguments {
            if !is_mutable_reference_to(self.cx, argument, "bevy_asset", "LoadBuilder") {
                continue;
            }
            if let Some(binding_id) = mutable_referent_binding(self.cx, argument) {
                // A resolved local borrow lets this call invalidate one tracked binding.
                let _removed_guard_count = self.guards_by_binding.remove(&binding_id);
            } else {
                // An unknown referent may alias any tracked builder.
                self.guards_by_binding.clear();
            }
        }
    }

    /// Stop tracking a mutable builder after assignment changes its identity.
    fn forget_reassigned_builder(&mut self, left: &Expr<'_>) {
        // Other assignment targets cannot be one direct LoadBuilder local.
        if !bevy_support::expression_has_type(self.cx, left, "bevy_asset", "LoadBuilder") {
            return;
        }
        let ExprKind::Path(path) = left.kind else {
            return;
        };
        let Res::Local(binding_id) = self.cx.typeck_results().qpath_res(&path, left.hir_id) else {
            return;
        };
        // The right-hand builder was visited before this removal, so only the old local is stale.
        let _removed_guard_count = self.guards_by_binding.remove(&binding_id);
    }

    /// Report a guard replacement only when the receiver's prior guard count is known.
    fn check_guard_call(&self, expression: &Expr<'_>, receiver: &Expr<'_>) {
        // A first guard is valid, and unknown builder state is not enough to diagnose.
        if !is_load_builder_method(self.cx, expression, receiver, "with_guard")
            || known_guard_count(self.cx, receiver, &self.guards_by_binding)
                .is_none_or(|guard_count| guard_count == 0)
        {
            return;
        }

        let ExprKind::MethodCall(segment, ..) = expression.kind else {
            return;
        };
        // Point at the replacing method name rather than the whole builder expression.
        self.cx.emit_span_lint(
            BEVY_LOAD_BUILDER_REPLACED_GUARD,
            segment.ident.span,
            rustc_errors::DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message(
                        "this second guard replaces an earlier guard on the same load builder",
                    )
                    .help("combine guards into one owned value, such as a tuple");
            }),
        );
    }
}

/// Return whether an expression passes a mutable reference to one exact type.
fn is_mutable_reference_to(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    crate_name: &str,
    type_name: &str,
) -> bool {
    let expression_type = cx.typeck_results().expr_ty_adjusted(expression);
    let ty::Ref(_, inner, Mutability::Mut) = expression_type.kind() else {
        return false;
    };
    bevy_support::type_is_named(cx, *inner, crate_name, type_name)
}

/// Return whether an expression has rustc's closure type.
fn is_closure_typed(cx: &LateContext<'_>, expression: &Expr<'_>) -> bool {
    // References can wrap a closure when an opaque API receives it.
    let mut expression_type = cx.typeck_results().expr_ty_adjusted(expression);
    while let ty::Ref(_, referent, _) = expression_type.kind() {
        expression_type = *referent;
    }
    matches!(expression_type.kind(), ty::Closure(..))
}

/// Return whether an opaque call receives a closure value directly.
fn has_closure_argument(cx: &LateContext<'_>, arguments: &[Expr<'_>]) -> bool {
    arguments
        .iter()
        .any(|argument| is_closure_typed(cx, argument))
}

/// Find the local binding directly borrowed by a mutable reference expression.
fn mutable_referent_binding(cx: &LateContext<'_>, expression: &Expr<'_>) -> Option<HirId> {
    // Peel only compiler temporaries and the explicit mutable borrow expression.
    let mut referent = expression;
    while let ExprKind::DropTemps(inner) | ExprKind::AddrOf(_, Mutability::Mut, inner) =
        referent.kind
    {
        referent = inner;
    }
    // Return a binding only when the borrowed place is a direct local path.
    let ExprKind::Path(path) = referent.kind else {
        return None;
    };
    let Res::Local(binding_id) = cx.typeck_results().qpath_res(&path, referent.hir_id) else {
        return None;
    };
    Some(binding_id)
}

/// Resolve the number of guards on one supported builder expression.
fn known_guard_count(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    guards_by_binding: &HashMap<HirId, usize>,
) -> Option<usize> {
    // Normalize compiler wrappers before resolving this builder's source.
    match expression.kind {
        ExprKind::DropTemps(inner) => known_guard_count(cx, inner, guards_by_binding),
        ExprKind::Path(path) => local_guard_count(cx, expression, path, guards_by_binding),
        ExprKind::MethodCall(_, receiver, _, _) => {
            method_guard_count(cx, expression, receiver, guards_by_binding)
        }
        // Other expression forms do not prove a builder origin or guard count.
        ExprKind::ConstBlock(_)
        | ExprKind::Array(_)
        | ExprKind::Call(..)
        | ExprKind::Use(..)
        | ExprKind::Tup(_)
        | ExprKind::Binary(..)
        | ExprKind::Unary(..)
        | ExprKind::Lit(_)
        | ExprKind::Cast(..)
        | ExprKind::Type(..)
        | ExprKind::Let(_)
        | ExprKind::If(..)
        | ExprKind::Loop(..)
        | ExprKind::Match(..)
        | ExprKind::Closure(_)
        | ExprKind::Block(..)
        | ExprKind::Assign(..)
        | ExprKind::AssignOp(..)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::AddrOf(..)
        | ExprKind::Break(..)
        | ExprKind::Continue(_)
        | ExprKind::Ret(_)
        | ExprKind::Become(_)
        | ExprKind::InlineAsm(_)
        | ExprKind::OffsetOf(..)
        | ExprKind::Struct(..)
        | ExprKind::Repeat(..)
        | ExprKind::Yield(..)
        | ExprKind::UnsafeBinderCast(..)
        | ExprKind::Err(_) => None,
    }
}

/// Resolve a tracked local binding's most recent known guard count.
fn local_guard_count(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    path: rustc_hir::QPath<'_>,
    guards_by_binding: &HashMap<HirId, usize>,
) -> Option<usize> {
    // Restrict lookup to resolved local variables rather than names or fields.
    let Res::Local(binding_id) = cx.typeck_results().qpath_res(&path, expression.hir_id) else {
        return None;
    };
    guards_by_binding.get(&binding_id).copied()
}

/// Resolve guard counts through the supported Bevy builder chain methods.
fn method_guard_count(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
    guards_by_binding: &HashMap<HirId, usize>,
) -> Option<usize> {
    // AssetServer creates a builder without a guard.
    if is_asset_server_method(cx, expression, receiver, "load_builder") {
        return Some(0);
    }

    // Derive wrapper state from the receiver before preserving or incrementing it.
    let receiver_count = known_guard_count(cx, receiver, guards_by_binding)?;
    if is_load_builder_method(cx, expression, receiver, "with_guard") {
        Some(receiver_count.saturating_add(1))
    } else if is_guard_preserving_method(cx, expression, receiver) {
        Some(receiver_count)
    } else {
        None
    }
}

/// Identify `LoadBuilder` methods that retain the receiver's existing guard count.
fn is_guard_preserving_method(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
) -> bool {
    // These methods return the builder without replacing or adding its guard.
    is_load_builder_method(cx, expression, receiver, "with_settings")
        || is_load_builder_method(cx, expression, receiver, "override_unapproved")
}

/// Resolve one exact Bevy method on a receiver with the expected public type.
fn is_bevy_method(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
    method_crate: &str,
    receiver_crate: &str,
    receiver_type: &str,
    method_name: &str,
) -> bool {
    // Require method syntax before querying the resolved method definition.
    if !matches!(expression.kind, ExprKind::MethodCall(..)) {
        return false;
    }
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expression.hir_id) else {
        return false;
    };
    cx.tcx.crate_name(def_id.krate).as_str() == method_crate
        && cx.tcx.item_name(def_id).as_str() == method_name
        && bevy_support::expression_has_type(cx, receiver, receiver_crate, receiver_type)
}

/// Resolve `AssetServer::load_builder` by its method and receiver definitions.
fn is_asset_server_method(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
    method_name: &str,
) -> bool {
    is_bevy_method(
        cx,
        expression,
        receiver,
        "bevy_asset",
        "bevy_asset",
        "AssetServer",
        method_name,
    )
}

/// Resolve a `LoadBuilder` method through Bevy's type metadata.
fn is_load_builder_method(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
    method_name: &str,
) -> bool {
    is_bevy_method(
        cx,
        expression,
        receiver,
        "bevy_asset",
        "bevy_asset",
        "LoadBuilder",
        method_name,
    )
}

/// Preserve state updates for the Bevy methods tracked by this lint.
fn is_tracked_load_builder_method(
    cx: &LateContext<'_>,
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
) -> bool {
    is_asset_server_method(cx, expression, receiver, "load_builder")
        || is_load_builder_method(cx, expression, receiver, "with_guard")
        || is_load_builder_method(cx, expression, receiver, "with_settings")
        || is_load_builder_method(cx, expression, receiver, "override_unapproved")
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
/// Tests lint behavior and Bevy's guard replacement behavior.
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    /// Counts when Bevy drops a load guard.
    struct GuardDropProbe(
        /// Records whether Bevy has dropped this guard.
        Arc<AtomicBool>,
    );

    impl Drop for GuardDropProbe {
        /// Record one guard drop.
        fn drop(&mut self) {
            self.0.store(true, AtomicOrdering::SeqCst);
        }
    }

    /// Replace a load guard without making the test function itself a lint fixture.
    fn replace_guard_for_test(
        builder: LoadBuilder<'_>,
        guard: impl Send + Sync + 'static,
    ) -> LoadBuilder<'_> {
        builder.with_guard(guard)
    }

    /// Build `AssetServer` for the runtime test without a panicking resource lookup.
    fn test_asset_server() -> Result<AssetServer, std::io::Error> {
        // AssetServer requires the task pool and AssetPlugin in this minimal App.
        let mut app = App::new();
        let app = app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_asset::AssetPlugin::default(),
        ));
        app.world()
            .get_resource::<AssetServer>()
            .cloned()
            .ok_or_else(|| std::io::Error::other("AssetPlugin did not insert AssetServer"))
    }

    /// Compare a guard's dropped state with the expectation at one sequence point.
    fn guard_matches_drop_state(guard_dropped: &AtomicBool, is_expected_dropped: bool) -> bool {
        // A mismatch means Bevy retained or dropped the guard at the wrong point.
        let is_dropped = guard_dropped.load(AtomicOrdering::SeqCst);
        is_dropped == is_expected_dropped
    }

    /// Attach one guard and record whether Bevy retains it before replacement.
    fn builder_with_first_guard<'a>(
        asset_server: &'a AssetServer,
        first_dropped: &Arc<AtomicBool>,
    ) -> (LoadBuilder<'a>, bool) {
        // Keep the probe alive while the builder owns its guard value.
        let builder = asset_server
            .load_builder()
            .with_guard(GuardDropProbe(Arc::clone(first_dropped)));
        let is_first_guard_alive = guard_matches_drop_state(first_dropped.as_ref(), false);
        (builder, is_first_guard_alive)
    }

    /// Replace a guard and record both guards' state immediately afterward.
    fn replace_and_check_guards<'a>(
        builder: LoadBuilder<'a>,
        first_dropped: &AtomicBool,
        second_dropped: &Arc<AtomicBool>,
    ) -> (LoadBuilder<'a>, bool) {
        // This helper keeps the deliberate replacement outside the lint fixture body.
        let builder = replace_guard_for_test(builder, GuardDropProbe(Arc::clone(second_dropped)));
        let is_first_guard_dropped = guard_matches_drop_state(first_dropped, true);
        let is_second_guard_alive = guard_matches_drop_state(second_dropped.as_ref(), false);
        (builder, is_first_guard_dropped && is_second_guard_alive)
    }

    /// Drop the final builder and report whether its guard was released.
    fn is_replacement_guard_dropped_after_builder_drop(
        builder: LoadBuilder<'_>,
        second_dropped: &AtomicBool,
    ) -> bool {
        // Final drop must release the guard still owned by the builder.
        drop(builder);
        guard_matches_drop_state(second_dropped, true)
    }

    /// Verify Bevy drops the first guard when a second one replaces it.
    #[test]
    fn replacing_guard_drops_previous_guard_immediately() -> Result<(), std::io::Error> {
        // Keep independent probes so replacement and final drop can be distinguished.
        let first_dropped = Arc::new(AtomicBool::new(false));
        let second_dropped = Arc::new(AtomicBool::new(false));
        let asset_server = test_asset_server()?;

        // Check each ownership transition at the moment it occurs.
        let (builder, is_first_guard_alive) =
            builder_with_first_guard(&asset_server, &first_dropped);
        let (builder, is_replacement_state_valid) =
            replace_and_check_guards(builder, first_dropped.as_ref(), &second_dropped);
        let is_replacement_guard_dropped =
            is_replacement_guard_dropped_after_builder_drop(builder, second_dropped.as_ref());

        let is_guard_drop_order_valid =
            is_first_guard_alive && is_replacement_state_valid && is_replacement_guard_dropped;
        if is_guard_drop_order_valid {
            Ok(())
        } else {
            Err(std::io::Error::other("Bevy guard drop order changed"))
        }
    }
}
