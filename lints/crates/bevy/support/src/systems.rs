//! Semantic analysis for Bevy systems, schedules, and app registration.

use super::helpers::{
    AppRunVisitor, adt_name, app_method_call, query_data_has_any_mut_ref, query_data_type,
    query_filter_has_camera, query_filter_type, system_expressions, system_function,
};
use super::{
    Body, DefId, Expr, ExprKind, FnDecl, LateContext, LocalDefId, RegisteredSystems, Span, Visitor,
    bevy_method_call, expression_has_type, trait_is_named, type_is_named,
};

/// Classification of resolved conditions on one system registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegistrationMode {
    /// The condition analysis does not prove that the system runs at most once.
    MayRepeat,
    /// A resolved condition guarantees that the system runs at most once.
    AtMostOnce,
}

/// Return local systems from one resolved `App::add_systems` call.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::directly_registered_systems(cx, expr);
/// };
/// ```
#[must_use]
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

/// Return local systems in a direct registration that may run repeatedly.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::directly_registered_repeating_systems(cx, expr);
/// };
/// ```
#[must_use]
pub fn directly_registered_repeating_systems(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<Vec<LocalDefId>> {
    // Resolve the app registration before traversing its system configuration.
    let call = app_method_call(cx, expr, "add_systems")?;
    let [_, systems, ..] = call.arguments else {
        return None;
    };
    Some(repeating_systems_in_config(
        cx,
        systems,
        RegistrationMode::MayRepeat,
    ))
}

/// Collect direct local systems unless their resolved run condition is one-shot.
fn repeating_systems_in_config(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    mode: RegistrationMode,
) -> Vec<LocalDefId> {
    // Apply an outer tuple condition to every configured member.
    if let ExprKind::Tup(elements) = expr.kind {
        return elements
            .iter()
            .flat_map(|element| repeating_systems_in_config(cx, element, mode))
            .collect();
    }

    // Preserve a proved condition through each resolved Bevy schedule wrapper.
    if let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind {
        let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
            return Vec::new();
        };
        let Some(schedule_configs) = cx.tcx.trait_of_assoc(method) else {
            return Vec::new();
        };
        if !trait_is_named(cx, schedule_configs, "bevy_ecs", "IntoScheduleConfigs") {
            return Vec::new();
        }

        // `run_if` and `distributive_run_if` attach their condition to this receiver.
        let is_run_condition_method = matches!(
            cx.tcx.item_name(method).as_str(),
            "run_if" | "distributive_run_if"
        );
        let mode = if is_run_condition_method
            && arguments
                .first()
                .is_some_and(|condition| condition_is_one_shot(cx, condition))
        {
            RegistrationMode::AtMostOnce
        } else {
            mode
        };
        return repeating_systems_in_config(cx, receiver, mode);
    }

    // Emit only local function paths that remain eligible for repeated execution.
    if mode == RegistrationMode::AtMostOnce {
        return Vec::new();
    }
    system_function(cx, expr)
        .and_then(DefId::as_local)
        .into_iter()
        .collect()
}

/// Return whether an expression is the resolved built-in `run_once` condition.
fn condition_is_one_shot(cx: &LateContext<'_>, condition: &Expr<'_>) -> bool {
    if let Some(def_id) = system_function(cx, condition) {
        // Match Bevy's exact definition path so same-named application functions do not count.
        let crate_name = cx.tcx.crate_name(def_id.krate);
        if crate_name.as_str() != "bevy_ecs" {
            return false;
        }
        let function_name = cx.tcx.item_name(def_id);
        if function_name.as_str() != "run_once" {
            return false;
        }
        let definition_path = cx.tcx.def_path(def_id);
        let mut path = definition_path
            .data
            .iter()
            .rev()
            .filter_map(|segment| segment.data.get_opt_name());
        return matches!(
            (path.next(), path.next(), path.next(), path.next()),
            (Some(function), Some(module), Some(parent), Some(schedule))
                if function.as_str() == "run_once"
                    && module.as_str() == "common_conditions"
                    && parent.as_str() == "condition"
                    && schedule.as_str() == "schedule"
        );
    }

    // Only conjunction preserves an at-most-once operand as an upper bound.
    let ExprKind::MethodCall(_, receiver, arguments, _) = condition.kind else {
        return false;
    };
    let Some(method) = cx.typeck_results().type_dependent_def_id(condition.hir_id) else {
        return false;
    };
    let Some(trait_id) = cx.tcx.trait_of_assoc(method) else {
        return false;
    };
    trait_is_named(cx, trait_id, "bevy_ecs", "SystemCondition")
        && matches!(
            cx.tcx.item_name(method).as_str(),
            "and" | "and_then" | "and_eager"
        )
        && (condition_is_one_shot(cx, receiver)
            || arguments
                .first()
                .is_some_and(|right| condition_is_one_shot(cx, right)))
}

/// Return whether a system function mutably queries camera-filtered entities.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id| {
///     let _ = bevy_support::is_system_mutably_querying_camera(cx, def_id);
/// };
/// ```
#[must_use]
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
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, schedule_name| {
///     let _ = bevy_support::disallowed_schedule_span(cx, expr, schedule_name);
/// };
/// ```
#[must_use]
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
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::inserted_message_resource_span(cx, expr);
/// };
/// ```
#[must_use]
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
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::iter_current_update_messages_span(cx, expr);
/// };
/// ```
#[must_use]
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
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, declaration, body, local_def_id| {
///     let _ = bevy_support::discarded_app_run_spans(cx, declaration, body, local_def_id);
/// };
/// ```
#[must_use]
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
