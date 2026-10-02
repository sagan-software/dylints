//! Semantic analysis for Bevy systems, schedules, and app registration.

use super::helpers::{
    AppRunVisitor, adt_name, app_method_call, query_data_has_any_mut_ref, query_data_type,
    query_filter_has_camera, query_filter_type, system_expressions, system_function,
};
use super::{
    Body, DefId, Expr, FnDecl, LateContext, LocalDefId, RegisteredSystems, Span, Visitor,
    bevy_method_call, expression_has_type, type_is_named,
};

/// Return local systems from one resolved `App::add_systems` call.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::directly_registered_systems(cx, expr);
/// };
/// ```
pub fn directly_registered_systems(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<RegisteredSystems> {
    // Resolve the registration call and preserve its schedule identity.
    let call = app_method_call(cx, expr, "add_systems")?;
    let [schedule, systems, ..] = call.arguments else {
        return None;
    };
    let schedule = adt_name(
        cx,
        cx.typeck_results().expr_ty_adjusted(schedule).peel_refs(),
    )?;
    // Keep only local function paths because later analysis needs their bodies.
    let systems = system_expressions(cx, systems)
        .into_iter()
        .filter_map(|system| system_function(cx, system)?.as_local())
        .collect::<Vec<_>>();
    (!systems.is_empty()).then_some(RegisteredSystems { schedule, systems })
}

/// Return whether a system function mutably queries camera-filtered entities.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id| {
///     let _ = bevy_support::is_system_mutably_querying_camera(cx, def_id);
/// };
/// ```
pub fn is_system_mutably_querying_camera(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx
        .fn_sig(def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .inputs()
        .skip_binder()
        .iter()
        .any(|input| {
            query_data_type(cx, *input).is_some_and(|data| query_data_has_any_mut_ref(data))
                && query_filter_type(cx, *input)
                    .is_some_and(|filter| query_filter_has_camera(cx, filter))
        })
}

/// Return spans for systems that mutate a camera in `FixedUpdate`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::camera_fixed_update_system_spans(cx, expr);
/// };
/// ```
pub fn camera_fixed_update_system_spans(cx: &LateContext<'_>, expr: &Expr<'_>) -> Vec<Span> {
    // Require the exact registration method and the fixed-update schedule.
    let Some(call) = app_method_call(cx, expr, "add_systems") else {
        return Vec::new();
    };
    let [schedule, systems, ..] = call.arguments else {
        return Vec::new();
    };
    if !expression_has_type(cx, schedule, "bevy_app", "FixedUpdate") {
        return Vec::new();
    }

    // Report only systems whose signatures prove mutable camera access.
    let mut spans = Vec::new();
    spans.extend(
        system_expressions(cx, systems)
            .into_iter()
            .filter(|system| {
                system_function(cx, system)
                    .is_some_and(|def_id| is_system_mutably_querying_camera(cx, def_id))
            })
            .map(|system| system.span),
    );
    spans
}

/// Return the disallowed schedule argument span for one `App::add_systems` call.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, schedule_name| {
///     let _ = bevy_support::disallowed_schedule_span(cx, expr, schedule_name);
/// };
/// ```
pub fn disallowed_schedule_span(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    schedule_name: &str,
) -> Option<Span> {
    let call = app_method_call(cx, expr, "add_systems")?;
    let schedule = call.arguments.first()?;
    expression_has_type(cx, schedule, "bevy_app", schedule_name).then_some(schedule.span)
}

/// Return the message-resource misuse span for `App::insert_resource` or `init_resource`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::inserted_message_resource_span(cx, expr);
/// };
/// ```
pub fn inserted_message_resource_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Handle value insertion before the generic type-based initialization form.
    if let Some(call) = app_method_call(cx, expr, "insert_resource") {
        let argument = call.arguments.first()?;
        return type_is_named(
            cx,
            cx.typeck_results().expr_ty_adjusted(argument),
            "bevy_ecs",
            "Messages",
        )
        .then_some(call.method_span);
    }

    let call = app_method_call(cx, expr, "init_resource")?;
    cx.typeck_results()
        .node_args(expr.hir_id)
        .types()
        .any(|ty| type_is_named(cx, ty, "bevy_ecs", "Messages"))
        .then_some(call.method_span)
}

/// Return the exact discouraged `Messages` iterator method span.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::iter_current_update_messages_span(cx, expr);
/// };
/// ```
pub fn iter_current_update_messages_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    bevy_method_call(
        cx,
        expr,
        "bevy_ecs",
        "Messages",
        "iter_current_update_messages",
    )
    .map(|call| call.method_span)
}

/// Return discarded `App::run` spans in the crate's unit-returning entry function.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, declaration, body, local_def_id| {
///     let _ = bevy_support::discarded_app_run_spans(cx, declaration, body, local_def_id);
/// };
/// ```
pub fn discarded_app_run_spans<'tcx>(
    cx: &LateContext<'tcx>,
    declaration: &FnDecl<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<Span> {
    // Only the resolved entry function with a unit declaration can be this entrypoint.
    let is_entry = cx
        .tcx
        .entry_fn(())
        .is_some_and(|(entry, _)| entry == local_def_id.to_def_id());
    if !is_entry
        || !matches!(
            declaration.output,
            rustc_hir::FnRetTy::DefaultReturn(_)
                | rustc_hir::FnRetTy::Return(rustc_hir::Ty {
                    kind: rustc_hir::TyKind::Tup([]),
                    ..
                })
        )
    {
        return Vec::new();
    }

    // Traverse the accepted entrypoint only after its signature has been proved.
    let mut visitor = AppRunVisitor {
        cx,
        spans: Vec::new(),
    };
    visitor.visit_expr(body.value);
    visitor.spans
}
