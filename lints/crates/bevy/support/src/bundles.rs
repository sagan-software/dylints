//! Semantic analysis for Bevy bundles, plugins, and time conversions.

use super::helpers::{
    adt_name, app_method_call, collect_unit_bundle_values, is_unit_literal,
    local_plugin_uses_default_uniqueness, resolved_trait_method,
};
use super::{Expr, ExprKind, LateContext, Span, bevy_method_call, ty};

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
    /// Span of `.add_plugins(..)` to delete when removal is exact.
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
