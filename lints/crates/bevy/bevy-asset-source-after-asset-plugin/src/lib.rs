#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks explicit Bevy asset source and web plugin registration order.
//! The lint resolves `App::add_plugins`, `AssetApp::register_asset_source`, and
//! plugin types, then follows direct local moves within each function body. It
//! reports source or web plugin registration after a proven `AssetPlugin` add.
//! Control-flow merges retain facts that hold on every path. Opaque mutable
//! escapes and closure calls invalidate affected facts. Plugin group contents
//! and arbitrary plugin bodies remain unexamined when their order is unknown.

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
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
use bevy::app::{App, TaskPoolPlugin};
#[cfg(test)]
use bevy::asset::{
    AssetApp, AssetPlugin, AssetServer,
    io::{AssetSourceBuilder, memory::MemoryAssetReader, web::WebAssetPlugin},
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_ASSET_SOURCE_AFTER_ASSET_PLUGIN,
    Warn,
    "checks Bevy asset source and web plugin ordering",
    BevyAssetSourceAfterAssetPlugin
}

impl<'tcx> LateLintPass<'tcx> for BevyAssetSourceAfterAssetPlugin {
    /// Check each function body for provable ordering on one `App` value.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _: Span,
        _: rustc_span::def_id::LocalDefId,
    ) {
        let mut visitor = AssetOrderingVisitor {
            cx,
            state: OrderingState::default(),
        };
        visitor.visit_body(body);
    }
}

/// Identifies one app value and tracks only explicit asset plugin additions.
#[derive(Clone, Default)]
struct OrderingState {
    /// App binding aliases that originate from a direct local move.
    aliases: HashMap<HirId, HirId>,
    /// App identities on which an explicit `AssetPlugin` was added.
    asset_plugins: HashSet<HirId>,
    /// Bindings whose app identity differs across control-flow paths.
    unknown_apps: HashSet<HirId>,
}

impl OrderingState {
    /// Retain only app facts that agree across both control-flow paths.
    fn intersect_with(&mut self, other: &Self) {
        // A plugin addition is known after a merge only if both paths contain it.
        self.asset_plugins
            .retain(|app_id| other.asset_plugins.contains(app_id));

        // An alias is valid after a merge only when both paths choose one identity.
        let alias_ids = self
            .aliases
            .keys()
            .chain(other.aliases.keys())
            .copied()
            .collect::<HashSet<_>>();
        for binding_id in alias_ids {
            let left_alias = self.aliases.get(&binding_id).copied();
            let right_alias = other.aliases.get(&binding_id).copied();
            if left_alias != right_alias {
                let _is_unknown_app_added = self.unknown_apps.insert(binding_id);
                let _removed_alias = self.aliases.remove(&binding_id);
            }
        }
        // Unknown identities dominate aliases accumulated from either path.
        self.unknown_apps.extend(other.unknown_apps.iter().copied());
        for binding_id in &self.unknown_apps {
            let _removed_alias = self.aliases.remove(binding_id);
        }
    }
}

/// Walks one body in evaluation order and records provable plugin additions.
struct AssetOrderingVisitor<'a, 'tcx> {
    /// Rustc context for resolved Bevy APIs and expression types.
    cx: &'a LateContext<'tcx>,
    /// Per-body app aliases and known explicit asset plugin additions.
    state: OrderingState,
}

impl<'tcx> Visitor<'tcx> for AssetOrderingVisitor<'_, 'tcx> {
    /// Track direct local aliases after their initializer has been evaluated.
    fn visit_stmt(&mut self, statement: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(local) = statement.kind {
            if let Some(initializer) = local.init {
                // Analyze initializer calls before associating its result with the binding.
                self.visit_expr(initializer);
                if let PatKind::Binding(_, binding_id, _, None) = local.pat.kind
                    && bevy_support::expression_has_type(self.cx, initializer, "bevy_app", "App")
                    && let Some(app_id) = self.app_identity(initializer)
                {
                    let _previous_alias = self.state.aliases.insert(binding_id, app_id);
                }
            }
            // Preserve source-order effects in a let-else block before leaving this statement.
            if let Some(else_block) = local.els {
                self.visit_block(else_block);
            }
            return;
        }
        intravisit::walk_stmt(self, statement);
    }

    /// Visit receivers and arguments before applying the outer app method.
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        // Handle stateful forms explicitly so evaluation order matches Rust execution.
        match expression.kind {
            ExprKind::MethodCall(_, receiver, arguments, _) => {
                self.visit_method_call(expression, receiver, arguments);
            }
            ExprKind::If(condition, then_expression, else_expression) => {
                self.visit_if(condition, then_expression, else_expression);
            }
            ExprKind::Match(scrutinee, arms, _) => self.visit_match(scrutinee, arms),
            ExprKind::Loop(block, ..) => self.visit_branch_block(block),
            ExprKind::Assign(left, right, _) => self.visit_assignment(left, right),
            ExprKind::Call(callee, arguments) => self.visit_opaque_call(callee, arguments),
            // Other expression forms use rustc's ordinary recursive traversal.
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

    /// Keep closure-local changes out of the enclosing function's app state.
    fn visit_nested_body(&mut self, _: rustc_hir::BodyId) {}
}

impl<'tcx> AssetOrderingVisitor<'_, 'tcx> {
    /// Visit a method call and invalidate facts passed to an opaque method.
    fn visit_method_call(
        &mut self,
        expression: &'tcx Expr<'tcx>,
        receiver: &'tcx Expr<'tcx>,
        arguments: &'tcx [Expr<'tcx>],
    ) {
        // Visit nested calls first so their state changes precede this method call.
        self.visit_expr(receiver);
        for argument in arguments {
            self.visit_expr(argument);
        }
        self.check_app_call(expression, receiver, arguments);
        // Unknown APIs may mutate borrowed apps or invoke closures over captured apps.
        if !is_tracked_app_method(expression, receiver, self.cx) {
            self.invalidate_escaped_apps(arguments);
            if is_closure_typed(self.cx, receiver) || has_closure_argument(self.cx, arguments) {
                self.forget_all_app_facts();
            }
        }
    }

    /// Visit an assignment before updating app identity facts.
    fn visit_assignment(&mut self, left: &'tcx Expr<'tcx>, right: &'tcx Expr<'tcx>) {
        // Resolve both sides before changing the destination binding's identity.
        self.visit_expr(left);
        self.visit_expr(right);
        self.update_app_assignment(left, right);
    }

    /// Merge app facts across both outcomes of a conditional expression.
    fn visit_if(
        &mut self,
        condition: &'tcx Expr<'tcx>,
        then_expression: &'tcx Expr<'tcx>,
        else_expression: Option<&'tcx Expr<'tcx>>,
    ) {
        // Condition effects happen before either branch begins.
        self.visit_expr(condition);
        let then_state = self.visit_branch(then_expression);
        let else_state = if let Some(expression) = else_expression {
            self.visit_branch(expression)
        } else {
            self.state.clone()
        };
        // Keep only state facts valid on both paths.
        self.state = then_state;
        self.state.intersect_with(&else_state);
    }

    /// Merge app facts across every arm of a match expression.
    fn visit_match(&mut self, scrutinee: &'tcx Expr<'tcx>, arms: &'tcx [rustc_hir::Arm<'tcx>]) {
        // Scrutinee effects are shared by all arms and precede arm evaluation.
        self.visit_expr(scrutinee);
        let baseline = self.state.clone();
        let mut arm_states = Vec::with_capacity(arms.len());
        for arm in arms {
            // Start each arm from the same facts before recording its independent result.
            self.state = baseline.clone();
            intravisit::walk_arm(self, arm);
            arm_states.push(self.state.clone());
        }
        // An empty arm list preserves the scrutinee state; otherwise intersect all arms.
        let mut merged_state = arm_states.pop().unwrap_or_else(|| baseline.clone());
        for arm_state in arm_states {
            merged_state.intersect_with(&arm_state);
        }
        self.state = merged_state;
    }

    /// Visit an opaque call before invalidating app facts passed by mutable reference.
    fn visit_opaque_call(&mut self, callee: &'tcx Expr<'tcx>, arguments: &'tcx [Expr<'tcx>]) {
        // Analyze argument expressions before the opaque call can mutate their referents.
        self.visit_expr(callee);
        for argument in arguments {
            self.visit_expr(argument);
        }
        // Invalidate captured facts when the call can execute an unknown closure body.
        if is_closure_typed(self.cx, callee) || has_closure_argument(self.cx, arguments) {
            self.forget_all_app_facts();
        }
        self.invalidate_escaped_apps(arguments);
    }

    /// Visit one conditional branch without carrying its changes afterward.
    fn visit_branch(&mut self, expression: &'tcx Expr<'tcx>) -> OrderingState {
        // Keep the incoming state available while collecting this branch's result.
        let previous_state = self.state.clone();
        self.visit_expr(expression);
        let branch_state = self.state.clone();
        self.state = previous_state;
        branch_state
    }

    /// Visit one loop body without assuming that it runs before later code.
    fn visit_branch_block(&mut self, block: &'tcx rustc_hir::Block<'tcx>) {
        // A loop may execute zero times, so merge its effects with the incoming state.
        let previous_state = self.state.clone();
        self.visit_block(block);
        let body_state = self.state.clone();
        self.state = previous_state;
        self.state.intersect_with(&body_state);
    }

    /// Update or invalidate a local app identity after an assignment.
    fn update_app_assignment(&mut self, left: &Expr<'_>, right: &Expr<'_>) {
        // Ignore assignments whose target cannot hold the Bevy app identity being tracked.
        if !bevy_support::expression_has_type(self.cx, left, "bevy_app", "App") {
            return;
        }
        let ExprKind::Path(path) = left.kind else {
            self.forget_all_app_facts();
            return;
        };
        let Res::Local(binding_id) = self.cx.typeck_results().qpath_res(&path, left.hir_id) else {
            self.forget_all_app_facts();
            return;
        };

        // A resolved right-hand identity safely restores tracking after branch uncertainty.
        if let Some(app_id) = self.app_identity(right) {
            let _is_unknown_app_removed = self.state.unknown_apps.remove(&binding_id);
            if app_id == binding_id {
                let _removed_alias = self.state.aliases.remove(&binding_id);
            } else {
                let _previous_alias = self.state.aliases.insert(binding_id, app_id);
            }
        } else {
            // A fresh or opaque right-hand value makes prior plugin facts stale.
            let _removed_alias = self.state.aliases.remove(&binding_id);
            let _is_asset_plugin_removed = self.state.asset_plugins.remove(&binding_id);
        }
    }

    /// Forget app identities when assignment targets cannot be resolved locally.
    fn forget_all_app_facts(&mut self) {
        self.state.aliases.clear();
        self.state.asset_plugins.clear();
        self.state.unknown_apps.clear();
    }

    /// Drop plugin facts for each mutable app reference passed to an opaque call.
    fn invalidate_escaped_apps(&mut self, arguments: &[Expr<'_>]) {
        // Inspect only mutable references with the exact Bevy `App` referent type.
        for argument in arguments {
            if !is_mutable_reference_to(self.cx, argument, "bevy_app", "App") {
                continue;
            }
            if let Some(binding_id) = mutable_referent_binding(self.cx, argument) {
                // A direct local borrow lets the visitor invalidate only that app identity.
                self.forget_escaped_app(binding_id);
            } else {
                // An unresolved referent could alias any tracked app value.
                self.forget_all_app_facts();
            }
        }
    }

    /// Reset the tracked identity and aliases for one mutable app binding.
    fn forget_escaped_app(&mut self, binding_id: HirId) {
        // Unknown identity means the binding may represent more than one tracked app.
        if self.state.unknown_apps.contains(&binding_id) {
            self.forget_all_app_facts();
            return;
        }

        let app_id = self
            .state
            .aliases
            .get(&binding_id)
            .copied()
            .unwrap_or(binding_id);
        // Remove plugin and alias facts attached to the escaped identity.
        let _is_asset_plugin_removed = self.state.asset_plugins.remove(&app_id);
        self.state
            .aliases
            .retain(|alias_id, identity| *alias_id != binding_id && *identity != app_id);
        let _is_unknown_binding_removed = self.state.unknown_apps.remove(&binding_id);
        let _is_unknown_identity_removed = self.state.unknown_apps.remove(&app_id);
    }

    /// Resolve explicit `App` methods and check their direct plugin arguments.
    fn check_app_call(
        &mut self,
        expression: &Expr<'_>,
        receiver: &Expr<'_>,
        arguments: &[Expr<'_>],
    ) {
        // A registered source after AssetPlugin cannot update the already-built server.
        if is_app_method(
            expression,
            receiver,
            self.cx,
            "bevy_asset",
            "register_asset_source",
        ) {
            if self
                .app_identity(receiver)
                .is_some_and(|app_id| self.state.asset_plugins.contains(&app_id))
            {
                self.cx.emit_span_lint(
                    BEVY_ASSET_SOURCE_AFTER_ASSET_PLUGIN,
                    method_span(expression),
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic = diagnostic
                            .primary_message("this asset source is registered after `AssetPlugin`")
                            .help("register the asset source before adding `AssetPlugin`");
                    }),
                );
            }
            return;
        }

        if !is_app_method(expression, receiver, self.cx, "bevy_app", "add_plugins") {
            return;
        }

        // Skip additions when the receiver's identity is no longer statically known.
        let Some(app_id) = self.app_identity(receiver) else {
            return;
        };
        let Some(argument) = arguments.first() else {
            return;
        };
        // Tuple fields are visited in source order because Bevy adds plugins in that order.
        self.visit_plugin_types(
            self.cx.typeck_results().expr_ty_adjusted(argument),
            app_id,
            expression.span,
        );
    }

    /// Apply known plugin types in tuple order and leave unknown types untouched.
    fn visit_plugin_types(&mut self, ty: Ty<'_>, app_id: HirId, span: Span) {
        if bevy_support::type_is_named(self.cx, ty, "bevy_asset", "AssetPlugin") {
            // Record only an explicit resolved plugin type.
            let _is_asset_plugin_recorded = self.state.asset_plugins.insert(app_id);
            return;
        }
        if bevy_support::type_is_named(self.cx, ty, "bevy_asset", "WebAssetPlugin") {
            if self.state.asset_plugins.contains(&app_id) {
                self.cx.emit_span_lint(
                    BEVY_ASSET_SOURCE_AFTER_ASSET_PLUGIN,
                    span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic = diagnostic
                            .primary_message("`WebAssetPlugin` is added after `AssetPlugin`")
                            .help("add `WebAssetPlugin` before `AssetPlugin`");
                    }),
                );
            }
            return;
        }
        if let ty::Tuple(fields) = ty.kind() {
            // Visit tuple members in their declared order to preserve plugin sequencing.
            for field in *fields {
                self.visit_plugin_types(field, app_id, span);
            }
        }
    }
}

impl AssetOrderingVisitor<'_, '_> {
    /// Resolve a stable identity through direct app methods and simple local aliases.
    fn app_identity(&self, expression: &Expr<'_>) -> Option<HirId> {
        // Normalize compiler-inserted wrappers before inspecting the source expression.
        match expression.kind {
            ExprKind::DropTemps(inner) => self.app_identity(inner),
            ExprKind::Path(path) => self.app_path_identity(expression, path),
            ExprKind::MethodCall(_, receiver, _, _) => {
                self.app_method_identity(expression, receiver)
            }
            // Temporary App values have stable identity only within this expression.
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
            | ExprKind::Err(_) => self.temporary_app_identity(expression),
        }
    }

    /// Resolve a local App binding unless a control-flow merge made it ambiguous.
    fn app_path_identity(
        &self,
        expression: &Expr<'_>,
        path: rustc_hir::QPath<'_>,
    ) -> Option<HirId> {
        // Type and resolution checks prevent source spelling from identifying unrelated values.
        if !bevy_support::expression_has_type(self.cx, expression, "bevy_app", "App") {
            return None;
        }
        let Res::Local(binding_id) = self.cx.typeck_results().qpath_res(&path, expression.hir_id)
        else {
            return None;
        };
        if self.state.unknown_apps.contains(&binding_id) {
            return None;
        }

        // Follow only a local move whose identity remains known.
        Some(
            self.state
                .aliases
                .get(&binding_id)
                .copied()
                .unwrap_or(binding_id),
        )
    }

    /// Follow a direct App-returning Bevy method to its receiver identity.
    fn app_method_identity(&self, expression: &Expr<'_>, receiver: &Expr<'_>) -> Option<HirId> {
        // Only these resolved methods are known to return the same App value.
        let is_identity_method =
            is_app_method(expression, receiver, self.cx, "bevy_app", "add_plugins")
                || is_app_method(
                    expression,
                    receiver,
                    self.cx,
                    "bevy_asset",
                    "register_asset_source",
                );
        if is_identity_method
            && bevy_support::expression_has_type(self.cx, expression, "bevy_app", "App")
        {
            return self.app_identity(receiver);
        }
        self.temporary_app_identity(expression)
    }

    /// Assign a temporary identity only to a resolved App-valued expression.
    fn temporary_app_identity(&self, expression: &Expr<'_>) -> Option<HirId> {
        // Unknown result types cannot be connected safely to a plugin registration.
        bevy_support::expression_has_type(self.cx, expression, "bevy_app", "App")
            .then_some(expression.hir_id)
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
    // References can wrap a closure when it is passed to an opaque API.
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
    // Peel only compiler temporaries and the mutable borrow introduced at this call.
    let mut referent = expression;
    while let ExprKind::DropTemps(inner) | ExprKind::AddrOf(_, Mutability::Mut, inner) =
        referent.kind
    {
        referent = inner;
    }
    // Keep invalidation local only when the final referent resolves to one local binding.
    let ExprKind::Path(path) = referent.kind else {
        return None;
    };
    let Res::Local(binding_id) = cx.typeck_results().qpath_res(&path, referent.hir_id) else {
        return None;
    };
    Some(binding_id)
}

/// Resolve an exact `App` method while allowing trait and receiver crates to differ.
fn is_app_method(
    expression: &Expr<'_>,
    receiver: &Expr<'_>,
    cx: &LateContext<'_>,
    method_crate: &str,
    method_name: &str,
) -> bool {
    // Reject non-method syntax before asking type checking for its resolved definition.
    if !matches!(expression.kind, ExprKind::MethodCall(..)) {
        return false;
    }
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expression.hir_id) else {
        return false;
    };
    cx.tcx.crate_name(def_id.krate).as_str() == method_crate
        && cx.tcx.item_name(def_id).as_str() == method_name
        && bevy_support::expression_has_type(cx, receiver, "bevy_app", "App")
}

/// Preserve state updates for the Bevy app methods tracked by this lint.
fn is_tracked_app_method(expression: &Expr<'_>, receiver: &Expr<'_>, cx: &LateContext<'_>) -> bool {
    is_app_method(expression, receiver, cx, "bevy_app", "add_plugins")
        || is_app_method(
            expression,
            receiver,
            cx,
            "bevy_asset",
            "register_asset_source",
        )
}

/// Return the method identifier span for a resolved method call.
const fn method_span(expression: &Expr<'_>) -> Span {
    let ExprKind::MethodCall(segment, ..) = expression.kind else {
        return expression.span;
    };
    segment.ident.span
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
/// Checks asset source activation against Bevy's runtime behavior.
mod tests {
    use super::*;

    /// Build an in-memory source for the ordering checks.
    fn source_builder() -> AssetSourceBuilder {
        AssetSourceBuilder::new(|| Box::new(MemoryAssetReader::default()))
    }

    /// Read `AssetServer` without relying on Bevy's panicking resource accessor.
    fn asset_server(app: &App) -> Result<&AssetServer, std::io::Error> {
        app.world()
            .get_resource::<AssetServer>()
            .ok_or_else(|| std::io::Error::other("AssetPlugin did not insert AssetServer"))
    }

    /// Register the source after the app has already built its asset server.
    fn register_source_for_test(app: &mut App) {
        let _registered_app = app.register_asset_source("custom", source_builder());
    }

    /// Add the web asset plugin after the app has already added `AssetPlugin`.
    fn install_web_plugin_for_test(app: &mut App) {
        let _configured_app = app.add_plugins(WebAssetPlugin::default());
    }

    /// Verify Bevy includes a source registered before `AssetPlugin` starts.
    #[test]
    fn source_before_asset_plugin_is_active() -> Result<(), std::io::Error> {
        // Build AssetServer only after the custom source has been inserted into App.
        let mut app = App::new();
        let app = app
            .add_plugins(TaskPoolPlugin::default())
            .register_asset_source("custom", source_builder())
            .add_plugins(AssetPlugin::default());

        let asset_server = asset_server(app)?;
        let _source = asset_server
            .get_source("custom")
            .map_err(std::io::Error::other)?;
        Ok(())
    }

    /// Verify a late source registration does not update the active server.
    #[test]
    fn source_after_asset_plugin_is_not_active() -> Result<(), std::io::Error> {
        // The helper isolates intentional late registration from this test's lint pass.
        let mut app = App::new();
        let app = app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
        register_source_for_test(app);

        // Inspect the server created before the helper performs late registration.
        let asset_server = asset_server(app)?;
        // A missing source confirms registration did not rebuild the active server.
        let is_source_missing = asset_server.get_source("custom").is_err();
        if is_source_missing {
            Ok(())
        } else {
            Err(std::io::Error::other(
                "late source unexpectedly became active",
            ))
        }
    }

    /// Verify Bevy registers web sources when its plugin precedes `AssetPlugin`.
    #[test]
    fn web_asset_plugin_before_asset_plugin_is_active() -> Result<(), std::io::Error> {
        // The web source is available only when its plugin precedes AssetPlugin.
        let mut app = App::new();
        let app = app.add_plugins((
            TaskPoolPlugin::default(),
            WebAssetPlugin::default(),
            AssetPlugin::default(),
        ));

        let asset_server = asset_server(app)?;
        let _source = asset_server
            .get_source("https")
            .map_err(std::io::Error::other)?;
        Ok(())
    }

    /// Verify late web source registration does not update the active server.
    #[test]
    fn web_asset_plugin_after_asset_plugin_is_not_active() -> Result<(), std::io::Error> {
        // The helper isolates intentional late plugin addition from this lint pass.
        let mut app = App::new();
        let app = app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
        install_web_plugin_for_test(app);

        // Inspect the server created before the helper adds the web plugin.
        let asset_server = asset_server(app)?;
        // A missing HTTPS source confirms the late plugin did not rebuild the server.
        let is_source_missing = asset_server.get_source("https").is_err();
        if is_source_missing {
            Ok(())
        } else {
            Err(std::io::Error::other(
                "late web source unexpectedly became active",
            ))
        }
    }
}
