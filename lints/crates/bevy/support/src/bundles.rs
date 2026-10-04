//! Semantic analysis for Bevy bundles, plugins, and time conversions.

use super::helpers::{
    LocalPluginUniqueness, adt_name, app_method_call, collect_unit_bundle_values, is_unit_literal,
    local_path_id, local_plugin_uniqueness, local_plugin_uses_default_uniqueness,
    resolved_trait_method,
};
use super::{Block, Expr, ExprKind, LateContext, Span, Visitor, bevy_method_call, ty};
use rustc_middle::hir::nested_filter::OnlyBodies;

/// One unit value passed inside a Bevy bundle.
#[derive(Clone, Debug)]
pub struct UnitBundleValue {
    /// Span of the unit expression.
    pub span: Span,
    /// Replacements that remove the unit value without changing the inserted components.
    ///
    /// The list is empty when no exact rewrite is known.
    pub replacements: Vec<(Span, String)>,
    /// Message that describes the replacements.
    pub suggestion: &'static str,
}

/// Return unit values passed to known Bevy bundle methods.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::unit_bundle_values(cx, expr);
/// };
/// ```
pub fn unit_bundle_values(cx: &LateContext<'_>, expr: &Expr<'_>) -> Vec<UnitBundleValue> {
    // Resolve only method calls before inspecting receiver-specific bundle behavior.
    let ExprKind::MethodCall(segment, receiver, [bundle, ..], _) = expr.kind else {
        return Vec::new();
    };
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return Vec::new();
    };
    if cx.tcx.crate_name(def_id.krate).as_str() != "bevy_ecs" {
        return Vec::new();
    }

    // Restrict the accepted API set to methods whose first argument is a bundle.
    let receiver_ty = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs();
    let method_symbol = cx.tcx.item_name(def_id);
    let method_name = method_symbol.as_str();
    let is_bundle_accepted = adt_name(cx, receiver_ty).is_some_and(|name| {
        matches!(
            name.as_str(),
            "Commands"
                | "World"
                | "EntityCommands"
                | "EntityWorldMut"
                | "RelatedSpawner"
                | "RelatedSpawnerCommands"
        ) && matches!(method_name, "spawn" | "insert" | "insert_if_new")
    });
    if !is_bundle_accepted {
        return Vec::new();
    }

    // A literal unit spawn is exactly `spawn_empty`, which every accepted spawner provides.
    if method_name == "spawn" && is_unit_literal(bundle) {
        let replacements = if expr.span.from_expansion() {
            Vec::new()
        } else {
            vec![(
                segment.ident.span.to(expr.span.shrink_to_hi()),
                String::from("spawn_empty()"),
            )]
        };
        return vec![UnitBundleValue {
            span: bundle.span,
            replacements,
            suggestion: "spawn the empty entity with `spawn_empty()`",
        }];
    }

    // Preserve every nested unit span so diagnostics can target the smallest expression.
    let mut values = Vec::new();
    collect_unit_bundle_values(cx.typeck_results().expr_ty(bundle), bundle, &mut values);
    values
}

/// Spans for an `add_plugins` call that repeats the previous plugin.
#[derive(Clone, Copy, Debug)]
pub struct DuplicatePluginAddition {
    /// Span of the repeated `add_plugins` method name.
    pub method_span: Span,
    /// Span to delete when removal is exact.
    pub removal: Option<Span>,
}

/// Return the duplicate `add_plugins` call for adjacent chained calls.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::duplicate_plugin_addition(cx, expr);
/// };
/// ```
pub fn duplicate_plugin_addition(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<DuplicatePluginAddition> {
    // Require adjacent resolved registration calls before comparing their arguments.
    let outer = app_method_call(cx, expr, "add_plugins")?;
    let inner = app_method_call(cx, outer.receiver, "add_plugins")?;
    let [outer_plugin] = outer.arguments else {
        return None;
    };
    let [inner_plugin] = inner.arguments else {
        return None;
    };
    let outer_ty = cx.typeck_results().expr_ty(outer_plugin);
    let inner_ty = cx.typeck_results().expr_ty(inner_plugin);

    // Default plugin uniqueness makes equal adjacent plugin types duplicates.
    if outer_ty != inner_ty || !local_plugin_uses_default_uniqueness(cx, outer_ty) {
        return None;
    }
    // Deleting a path argument cannot drop side effects; other arguments might run code.
    let removal = (matches!(outer_plugin.kind, ExprKind::Path(_))
        && !expr.span.from_expansion()
        && !outer.receiver.span.from_expansion())
    .then(|| {
        outer
            .receiver
            .span
            .shrink_to_hi()
            .to(expr.span.shrink_to_hi())
    });
    Some(DuplicatePluginAddition {
        method_span: outer.method_span,
        removal,
    })
}

/// Return the method span for a tuple that repeats one local unique plugin type.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::tuple_duplicate_plugin_addition(cx, expr);
/// };
/// ```
pub fn tuple_duplicate_plugin_addition(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Resolve only Bevy's App or SubApp registration method.
    let call = app_method_call(cx, expr, "add_plugins")?;
    let [plugins] = call.arguments else {
        return None;
    };

    // Report a repeated local plugin type without deleting tuple elements that may have effects.
    tuple_has_duplicate_unique_plugin(cx, cx.typeck_results().expr_ty(plugins))
        .then_some(call.method_span)
}

/// Return repeated direct local plugin additions within one block.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, block| {
///     let _ = bevy_support::duplicate_plugin_additions_in_block(cx, block);
/// };
/// ```
pub fn duplicate_plugin_additions_in_block<'tcx>(
    cx: &LateContext<'tcx>,
    block: &Block<'tcx>,
) -> Vec<DuplicatePluginAddition> {
    let mut added_plugins: Vec<(rustc_hir::HirId, Vec<super::Ty<'tcx>>)> = Vec::new();
    let mut duplicates = Vec::new();
    // Walk statements in source order so each tracked binding represents one proven App value.
    for statement in block.stmts {
        if let rustc_hir::StmtKind::Semi(expr) = statement.kind
            && let Some(call) = app_method_call(cx, expr, "add_plugins")
            && let Some(app_binding) = local_path_id(cx, call.receiver)
        {
            // Tuple diagnostics cover duplicates within one call; do not report them twice.
            if tuple_duplicate_plugin_addition(cx, expr).is_some() {
                added_plugins.retain(|(binding, _)| *binding != app_binding);
                continue;
            }

            // Unknown groups, external plugins, and uniqueness overrides invalidate the history.
            let [plugins] = call.arguments else {
                added_plugins.retain(|(binding, _)| *binding != app_binding);
                continue;
            };
            let Some(plugin_types) =
                known_direct_plugin_types(cx, cx.typeck_results().expr_ty(plugins))
            else {
                added_plugins.retain(|(binding, _)| *binding != app_binding);
                continue;
            };

            if let Some(duplicate) = remember_plugin_types(
                &mut added_plugins,
                app_binding,
                plugin_types,
                call.method_span,
            ) {
                duplicates.push(duplicate);
            }
            continue;
        }

        // Any other use may mutate, reassign, or alias the App, so forget its prior plugins.
        let mut binding_uses = LocalBindingUseCollector {
            cx,
            bindings: Vec::new(),
        };
        binding_uses.visit_stmt(statement);
        added_plugins.retain(|(binding, _)| !binding_uses.bindings.contains(binding));
    }
    duplicates
}

/// Add known plugin types to one App history and return its first repeated type.
fn remember_plugin_types<'tcx>(
    histories: &mut Vec<(rustc_hir::HirId, Vec<super::Ty<'tcx>>)>,
    app_binding: rustc_hir::HirId,
    plugin_types: Vec<super::Ty<'tcx>>,
    method_span: Span,
) -> Option<DuplicatePluginAddition> {
    // Keep independent App bindings in separate histories.
    let Some((_, prior_types)) = histories
        .iter_mut()
        .find(|(binding, _)| *binding == app_binding)
    else {
        histories.push((app_binding, plugin_types));
        return None;
    };

    // Help only because evaluating a later plugin expression may have effects.
    let mut repeated = false;
    for plugin_ty in plugin_types {
        if prior_types.contains(&plugin_ty) {
            repeated = true;
        } else {
            prior_types.push(plugin_ty);
        }
    }
    repeated.then_some(DuplicatePluginAddition {
        method_span,
        removal: None,
    })
}

/// Return known unique plugin types for a local plugin or a tuple of local plugins.
fn known_direct_plugin_types<'tcx>(
    cx: &LateContext<'tcx>,
    plugins_ty: super::Ty<'tcx>,
) -> Option<Vec<super::Ty<'tcx>>> {
    if let ty::Tuple(elements) = plugins_ty.kind() {
        let mut plugin_types = Vec::new();
        // Every tuple member must have a local, default-unique `Plugin` implementation.
        for plugin_ty in *elements {
            if local_plugin_uniqueness(cx, plugin_ty) != Some(LocalPluginUniqueness::DefaultUnique)
            {
                return None;
            }
            plugin_types.push(plugin_ty);
        }
        return Some(plugin_types);
    }

    // A plugin with an explicit uniqueness override can make arbitrary registry changes.
    (local_plugin_uniqueness(cx, plugins_ty) == Some(LocalPluginUniqueness::DefaultUnique))
        .then(|| vec![plugins_ty])
}

/// Collect local bindings referenced by one statement before invalidating App provenance.
struct LocalBindingUseCollector<'a, 'tcx> {
    /// Type-checking context used to resolve local paths.
    cx: &'a LateContext<'tcx>,
    /// Local binding IDs referenced by the statement.
    bindings: Vec<rustc_hir::HirId>,
}

impl<'tcx> Visitor<'tcx> for LocalBindingUseCollector<'_, 'tcx> {
    /// Include closure and async bodies when invalidating App provenance.
    type NestedFilter = OnlyBodies;

    /// Type-checking context for visiting nested bodies.
    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    /// Record local paths and continue through nested expressions and blocks.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let Some(binding) = local_path_id(self.cx, expr) {
            self.bindings.push(binding);
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Return whether one tuple contains the same default-unique local plugin type twice.
fn tuple_has_duplicate_unique_plugin(cx: &LateContext<'_>, plugins_ty: super::Ty<'_>) -> bool {
    let ty::Tuple(elements) = plugins_ty.kind() else {
        return false;
    };
    let mut unique_plugin_types = Vec::new();
    // Compare only local plugins whose default uniqueness is established from their impl.
    for plugin_ty in *elements {
        if local_plugin_uniqueness(cx, plugin_ty) != Some(LocalPluginUniqueness::DefaultUnique) {
            continue;
        }
        if unique_plugin_types.contains(&plugin_ty) {
            return true;
        }
        unique_plugin_types.push(plugin_ty);
    }
    false
}

/// A widening of `Time::elapsed_secs()` to `f64`.
#[derive(Clone, Debug)]
pub struct ElapsedSecsWidening {
    /// Span of the `elapsed_secs` method name.
    pub method_span: Span,
    /// Spans of the conversion syntax to delete when the rewrite is exact.
    ///
    /// The list is empty when the expression comes from a macro expansion.
    pub removals: Vec<Span>,
}

/// Return a `Time::elapsed_secs()` value widened to `f64` by `as`, `From`, or `Into`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::elapsed_secs_widening(cx, expr);
/// };
/// ```
pub fn elapsed_secs_widening(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<ElapsedSecsWidening> {
    // Every accepted form produces an `f64` from one `elapsed_secs` call.
    if !matches!(
        cx.typeck_results().expr_ty(expr).kind(),
        ty::Float(ty::FloatTy::F64)
    ) {
        return None;
    }
    let (operand, removals) = if let ExprKind::Cast(operand, _) = expr.kind {
        // `time.elapsed_secs() as f64`
        (
            operand,
            vec![operand.span.between(expr.span.shrink_to_hi())],
        )
    } else if let ExprKind::Call(function, [argument]) = expr.kind
        && resolved_trait_method(cx, function, rustc_span::sym::From)
    {
        // `f64::from(time.elapsed_secs())`
        (
            argument,
            vec![
                expr.span.shrink_to_lo().to(argument.span.shrink_to_lo()),
                argument.span.between(expr.span.shrink_to_hi()),
            ],
        )
    } else if let ExprKind::MethodCall(_, receiver, [], _) = expr.kind {
        let is_into_conversion = cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .and_then(|def_id| cx.tcx.trait_of_assoc(def_id))
            .is_some_and(|def_id| cx.tcx.is_diagnostic_item(rustc_span::sym::Into, def_id));
        if !is_into_conversion {
            return None;
        }
        // `time.elapsed_secs().into()`
        (
            receiver,
            vec![receiver.span.between(expr.span.shrink_to_hi())],
        )
    } else {
        return None;
    };
    let call = bevy_method_call(cx, operand, "bevy_time", "Time", "elapsed_secs")?;
    // Macro-produced syntax has no reliable source text to rewrite.
    let removals = if expr.span.from_expansion() || operand.span.from_expansion() {
        Vec::new()
    } else {
        removals
    };
    Some(ElapsedSecsWidening {
        method_span: call.method_span,
        removals,
    })
}
