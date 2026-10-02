//! Semantic Bevy method and type resolution helpers.

use super::{BevyMethodCall, DefId, Expr, ExprKind, LateContext, Ty, ty};

/// Resolve an exact Bevy method on a named receiver type.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, receiver_crate, receiver_name, method_name| {
///     let _ = bevy_support::bevy_method_call(cx, expr, receiver_crate, receiver_name, method_name);
/// };
/// ```
pub fn bevy_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    receiver_crate: &str,
    receiver_name: &str,
    method_name: &str,
) -> Option<BevyMethodCall<'hir>> {
    // Reject unrelated expression shapes before asking rustc for method metadata.
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    // Match the resolved crate, method, and receiver type as one semantic boundary.
    if cx.tcx.crate_name(def_id.krate).as_str() != receiver_crate
        || cx.tcx.item_name(def_id).as_str() != method_name
        || !expression_has_type(cx, receiver, receiver_crate, receiver_name)
    {
        return None;
    }

    Some(BevyMethodCall {
        receiver,
        arguments,
        method_span: segment.ident.span,
        def_id,
    })
}

/// Resolve an exact `World` method.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, method_name| {
///     let _ = bevy_support::world_method_call(cx, expr, method_name);
/// };
/// ```
pub fn world_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    method_name: &str,
) -> Option<BevyMethodCall<'hir>> {
    bevy_method_call(cx, expr, "bevy_ecs", "World", method_name)
}

/// Return whether an expression has one exact ADT type.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, crate_name, type_name| {
///     let _ = bevy_support::expression_has_type(cx, expr, crate_name, type_name);
/// };
/// ```
pub fn expression_has_type(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    crate_name: &str,
    type_name: &str,
) -> bool {
    type_is_named(
        cx,
        cx.typeck_results().expr_ty_adjusted(expr).peel_refs(),
        crate_name,
        type_name,
    )
}

/// Return whether a type is one exact ADT.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, ty, crate_name, type_name| {
///     let _ = bevy_support::type_is_named(cx, ty, crate_name, type_name);
/// };
/// ```
pub fn type_is_named(cx: &LateContext<'_>, ty: Ty<'_>, crate_name: &str, type_name: &str) -> bool {
    let ty::Adt(definition, _) = ty.peel_refs().kind() else {
        return false;
    };
    let def_id = definition.did();

    cx.tcx.crate_name(def_id.krate).as_str() == crate_name
        && cx.tcx.item_name(def_id).as_str() == type_name
}

/// Return whether a definition is one exact trait.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id, crate_name, trait_name| {
///     let _ = bevy_support::trait_is_named(cx, def_id, crate_name, trait_name);
/// };
/// ```
pub fn trait_is_named(
    cx: &LateContext<'_>,
    def_id: DefId,
    crate_name: &str,
    trait_name: &str,
) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == crate_name
        && cx.tcx.item_name(def_id).as_str() == trait_name
}
