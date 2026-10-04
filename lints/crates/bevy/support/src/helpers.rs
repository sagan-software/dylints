//! Shared traversal, type-shape, and implementation helpers for Bevy analyses.

use super::{
    BevyMethodCall, Body, ControlFlow, DefId, Expr, ExprKind, FnDecl, FnKind, ItemKind,
    LateContext, LocalDefId, MarkerTrait, Mutability, Reborrowable, Region, Res, Size, Span,
    Symbol, Ty, TyAndLayout, TyCtxt, TypeVisitor, UnitBundleValue, VariantData, Visitor,
    bevy_method_call, expression_has_type, trait_is_named, ty, type_is_named,
};
use rustc_hir::PatKind;
use rustc_middle::ty::layout::LayoutOf;

/// Return indexed parameter types for a function whose signature
/// the author controls.
///
/// Trait implementation methods yield nothing because traits fix their
/// parameter types.
pub(crate) fn adjustable_inputs<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<(usize, Ty<'tcx>)> {
    // A trait implementation cannot change the parameter types the trait declares.
    if cx
        .tcx
        .trait_impl_of_assoc(local_def_id.to_def_id())
        .is_some()
    {
        return Vec::new();
    }
    let signature = match kind {
        FnKind::Closure => cx.tcx.closure_user_provided_sig(local_def_id).value,
        FnKind::ItemFn(..) | FnKind::Method(..) => cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    };
    signature
        .skip_binder()
        .inputs()
        .iter()
        .copied()
        .enumerate()
        .collect()
}

/// Map parameter indexes to the spans of their declared types.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |declaration, indexes: Vec<usize>| {
///     let _ = bevy_support::parameter_spans(declaration, indexes);
/// };
/// ```
pub fn parameter_spans<'hir>(
    declaration: &'hir FnDecl<'hir>,
    indexes: impl IntoIterator<Item = usize>,
) -> impl Iterator<Item = Span> {
    parameter_types(declaration, indexes).map(|input| input.span)
}

/// Map parameter indexes to their declared HIR types.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |declaration, indexes: Vec<usize>| {
///     let _ = bevy_support::parameter_types(declaration, indexes);
/// };
/// ```
pub fn parameter_types<'hir>(
    declaration: &'hir FnDecl<'hir>,
    indexes: impl IntoIterator<Item = usize>,
) -> impl Iterator<Item = &'hir rustc_hir::Ty<'hir>> {
    indexes
        .into_iter()
        .filter_map(|index| declaration.inputs.get(index))
}

/// Return the closure or coroutine body behind a nested body identifier.
///
/// Visitors use it to analyze closures with the enclosing function while
/// skipping constant bodies, which have separate type-check results.
pub(crate) fn closure_body<'tcx>(
    cx: &LateContext<'tcx>,
    body_id: rustc_hir::BodyId,
) -> Option<&'tcx Body<'tcx>> {
    let owner = cx.tcx.hir_body_owner_def_id(body_id);
    matches!(
        cx.tcx.hir_body_owner_kind(owner),
        rustc_hir::BodyOwnerKind::Closure
    )
    .then(|| cx.tcx.hir_body(body_id))
}

/// Return query data from an instantiated `Query` type.
pub(crate) fn query_data_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    type_is_named(cx, ty, "bevy_ecs", "Query")
        .then(|| query_type_arguments(ty).next())
        .flatten()
}

/// Return query filters from an instantiated `Query` type.
pub(crate) fn query_filter_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    type_is_named(cx, ty, "bevy_ecs", "Query")
        .then(|| query_type_arguments(ty).nth(1))
        .flatten()
}

/// Return the type arguments of an ADT after peeling references.
pub(crate) fn query_type_arguments(ty: Ty<'_>) -> impl Iterator<Item = Ty<'_>> {
    let arguments = if let ty::Adt(_, arguments) = ty.peel_refs().kind() {
        arguments.types().collect()
    } else {
        Vec::new()
    };
    arguments.into_iter()
}

/// Return the binding introduced by a plain identifier parameter.
pub(crate) fn parameter_binding(body: &Body<'_>, index: usize) -> Option<rustc_hir::HirId> {
    let parameter = body.params.get(index)?;
    let PatKind::Binding(_, binding_id, _, None) = parameter.pat.kind else {
        return None;
    };
    Some(binding_id)
}

/// Collect `&mut` prefix replacements from HIR query data made of references and tuples.
pub(crate) fn collect_shared_reference_replacements(
    cx: &LateContext<'_>,
    data: &rustc_hir::Ty<'_>,
    replacements: &mut Vec<(Span, String)>,
) {
    if let rustc_hir::TyKind::Ref(_, mutable) = data.kind
        && mutable.mutbl == Mutability::Mut
    {
        // Rebuild the prefix from its tokens so an explicit lifetime survives.
        let prefix = data.span.until(mutable.ty.span);
        let Ok(text) = cx.tcx.sess.source_map().span_to_snippet(prefix) else {
            return;
        };
        let mut tokens = text.trim_start_matches('&').split_whitespace();
        let replacement = match (tokens.next(), tokens.next(), tokens.next()) {
            (Some("mut"), None, None) => String::from("&"),
            (Some(lifetime), Some("mut"), None) if lifetime.starts_with('\'') => {
                format!("&{lifetime} ")
            }
            _ => return,
        };
        replacements.push((prefix, replacement));
        return;
    }
    if let rustc_hir::TyKind::Tup(elements) = data.kind {
        for element in elements {
            collect_shared_reference_replacements(cx, element, replacements);
        }
    }
}

/// Return direct reference leaves from tuple query data.
pub(crate) fn direct_query_refs(ty: Ty<'_>) -> impl Iterator<Item = Ty<'_>> {
    let mut references = Vec::new();
    collect_direct_query_refs(ty, &mut references);
    references.into_iter()
}

/// Collect direct shared or mutable reference leaves from tuple query data.
pub(crate) fn collect_direct_query_refs<'tcx>(ty: Ty<'tcx>, references: &mut Vec<Ty<'tcx>>) {
    // Record a direct reference leaf and stop descending that branch.
    if let ty::Ref(_, inner, _) = ty.kind() {
        references.push(*inner);
        return;
    }
    // Recurse through tuple query data while preserving element order.
    if let ty::Tuple(elements) = ty.kind() {
        for element in *elements {
            collect_direct_query_refs(element, references);
        }
    }
}

/// Return mutable reference leaves from tuple query data.
pub(crate) fn query_data_mut_components(ty: Ty<'_>) -> impl Iterator<Item = Ty<'_>> {
    let mut components = Vec::new();
    collect_query_data_mut_components(ty, &mut components);
    components.into_iter()
}

/// Collect mutable reference leaves from tuple query data.
pub(crate) fn collect_query_data_mut_components<'tcx>(
    ty: Ty<'tcx>,
    components: &mut Vec<Ty<'tcx>>,
) {
    // Record a direct mutable reference leaf and stop descending that branch.
    if let ty::Ref(_, inner, Mutability::Mut) = ty.kind() {
        components.push(*inner);
        return;
    }
    // Recurse through tuple query data while preserving element order.
    if let ty::Tuple(elements) = ty.kind() {
        for element in *elements {
            collect_query_data_mut_components(element, components);
        }
    }
}

/// Return whether query data includes any mutable reference.
pub(crate) fn query_data_has_any_mut_ref(ty: Ty<'_>) -> bool {
    query_data_mut_components(ty).next().is_some()
}

/// Return whether a query filter includes `With<Camera>`.
pub(crate) fn query_filter_has_camera(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    // Search tuple filters recursively before inspecting one ADT filter.
    if let ty::Tuple(elements) = ty.kind() {
        return elements
            .iter()
            .any(|element| query_filter_has_camera(cx, element));
    }
    type_is_named(cx, ty, "bevy_ecs", "With")
        && query_type_arguments(ty)
            .any(|argument| type_is_named(cx, argument, "bevy_camera", "Camera"))
}

/// Return the local ADT definition represented by a type.
pub(crate) fn local_adt_id(ty: Ty<'_>) -> Option<LocalDefId> {
    let ty::Adt(definition, _) = ty.peel_refs().kind() else {
        return None;
    };
    definition.did().as_local()
}

/// Return the names of fields on a local named-field struct.
pub(crate) fn local_named_fields(cx: &LateContext<'_>, local_def_id: LocalDefId) -> Vec<Symbol> {
    let ItemKind::Struct(_, _, VariantData::Struct { fields, .. }) =
        cx.tcx.hir_expect_item(local_def_id).kind
    else {
        return Vec::new();
    };
    fields.iter().map(|field| field.ident.name).collect()
}

/// Return the layout size of one monomorphic local type.
pub(crate) fn local_type_size(cx: &LateContext<'_>, local_def_id: LocalDefId) -> Option<u64> {
    let ty = cx
        .tcx
        .type_of(local_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    cx.layout_of(ty).ok().map(|layout| layout.size.bytes())
}

/// Count all and mutable component accesses in query data.
pub(crate) fn query_access_width<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    query_data_targets: &[LocalDefId],
) -> (usize, usize) {
    // Count reference leaves directly, including their mutability.
    if let ty::Ref(_, _, mutability) = ty.kind() {
        return (1, usize::from(*mutability == Mutability::Mut));
    }
    // Sum nested tuple widths before resolving custom QueryData fields.
    if let ty::Tuple(elements) = ty.kind() {
        return elements.iter().fold((0, 0), |(total, mutable), element| {
            let (element_total, element_mutable) =
                query_access_width(cx, element, query_data_targets);
            (total + element_total, mutable + element_mutable)
        });
    }
    if let ty::Adt(definition, arguments) = ty.kind()
        && definition
            .did()
            .as_local()
            .is_some_and(|target| query_data_targets.contains(&target))
    {
        return definition.non_enum_variant().fields.iter().fold(
            (0, 0),
            |(total, mutable), field| {
                let (field_total, field_mutable) = query_access_width(
                    cx,
                    field.ty(cx.tcx, arguments).skip_norm_wip(),
                    query_data_targets,
                );
                (total + field_total, mutable + field_mutable)
            },
        );
    }
    (0, 0)
}

/// Return a listed component tracked by a `Changed` query filter.
pub(crate) fn query_filter_tracked_component(
    cx: &LateContext<'_>,
    ty: Ty<'_>,
    components: &[LocalDefId],
) -> Option<LocalDefId> {
    // Search tuple filters before matching a direct Changed filter.
    if let ty::Tuple(elements) = ty.kind() {
        return elements
            .iter()
            .find_map(|element| query_filter_tracked_component(cx, element, components));
    }
    if !type_is_named(cx, ty, "bevy_ecs", "Changed") {
        return None;
    }
    query_type_arguments(ty)
        .find_map(|argument| local_adt_id(argument).filter(|target| components.contains(target)))
}

/// Return item structs yielded by a local `QueryData` type and its read-only form.
pub(crate) fn query_data_item_types(cx: &LateContext<'_>, target: LocalDefId) -> Vec<DefId> {
    let mut items = Vec::new();
    let mut pending = vec![target.to_def_id()];
    // Follow `ReadOnly` to its own implementation so both item structs count.
    while let Some(query_data) = pending.pop() {
        let Some(implementation) = local_trait_implementations(cx, "bevy_ecs", "QueryData")
            .into_iter()
            .find(|(_, self_def_id)| *self_def_id == query_data)
        else {
            continue;
        };
        for item in cx
            .tcx
            .associated_items(implementation.0)
            .in_definition_order()
        {
            if !matches!(item.kind, ty::AssocKind::Type { .. }) {
                continue;
            }
            let ty::Adt(definition, _) = cx
                .tcx
                .type_of(item.def_id)
                .instantiate_identity()
                .skip_norm_wip()
                .kind()
            else {
                continue;
            };
            let def_id = definition.did();
            match item.name().as_str() {
                "Item" if !items.contains(&def_id) => items.push(def_id),
                "ReadOnly" if def_id != query_data => pending.push(def_id),
                _ => {}
            }
        }
    }
    items
}

/// Return names of fields read from values whose type is one of `owners`.
pub(crate) fn used_field_names<'tcx>(
    cx: &LateContext<'tcx>,
    body: &Body<'tcx>,
    owners: &[DefId],
) -> Vec<Symbol> {
    let mut visitor = FieldUseVisitor {
        cx,
        owners,
        names: Vec::new(),
    };
    visitor.visit_expr(body.value);
    visitor.names
}

/// Return the system expressions inside an `add_systems` argument.
///
/// Tuples are flattened, and schedule configuration methods such as `run_if`,
/// `chain`, or `after` are looked through to the systems they configure.
pub(crate) fn system_expressions<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Vec<&'hir Expr<'hir>> {
    if let ExprKind::Tup(elements) = expr.kind {
        return elements
            .iter()
            .flat_map(|element| system_expressions(cx, element))
            .collect();
    }
    if let ExprKind::MethodCall(_, receiver, _, _) = expr.kind {
        let is_schedule_config_method = cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .and_then(|def_id| cx.tcx.trait_of_assoc(def_id))
            .is_some_and(|def_id| trait_is_named(cx, def_id, "bevy_ecs", "IntoScheduleConfigs"));
        if is_schedule_config_method {
            return system_expressions(cx, receiver);
        }
    }
    vec![expr]
}

/// Return the function a system path expression names.
pub(crate) fn system_function(cx: &LateContext<'_>, system: &Expr<'_>) -> Option<DefId> {
    let ExprKind::Path(ref path) = system.kind else {
        return None;
    };
    if let Res::Def(rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn, def_id) =
        cx.typeck_results().qpath_res(path, system.hir_id)
    {
        Some(def_id)
    } else {
        None
    }
}

/// Resolve an exact `App` or `SubApp` method.
pub(crate) fn app_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    method_name: &str,
) -> Option<BevyMethodCall<'hir>> {
    bevy_method_call(cx, expr, "bevy_app", "App", method_name)
        .or_else(|| bevy_method_call(cx, expr, "bevy_app", "SubApp", method_name))
}

/// Return an ADT's item name.
pub(crate) fn adt_name(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Symbol> {
    let ty::Adt(definition, _) = ty.kind() else {
        return None;
    };
    Some(cx.tcx.item_name(definition.did()))
}

/// Return the name of a method call resolved to a `bevy_ecs` definition.
pub(crate) fn bevy_ecs_method_name(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Symbol> {
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    (cx.tcx.crate_name(def_id.krate).as_str() == "bevy_ecs").then(|| cx.tcx.item_name(def_id))
}

/// Return whether a callee path resolves to a method of one diagnostic-item trait.
pub(crate) fn resolved_trait_method(
    cx: &LateContext<'_>,
    callee: &Expr<'_>,
    trait_item: Symbol,
) -> bool {
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return false;
    };
    cx.tcx
        .trait_of_assoc(def_id)
        .is_some_and(|trait_def_id| cx.tcx.is_diagnostic_item(trait_item, trait_def_id))
}

/// Return whether a type has a zero-sized layout.
pub(crate) fn is_zst<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(
        cx.layout_of(ty),
        Ok(TyAndLayout { layout, .. }) if layout.size() == Size::ZERO
    )
}

/// Recognize one supported reborrowable proxy.
pub(crate) fn reborrowable_type(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Reborrowable> {
    // Peel references and require a resolved ADT before matching proxy types.
    let ty::Adt(definition, _) = ty.peel_refs().kind() else {
        return None;
    };
    let def_id = definition.did();
    let crate_symbol = cx.tcx.crate_name(def_id.krate);
    let type_symbol = cx.tcx.item_name(def_id);

    // Keep the allowlist closed so unrelated names cannot gain reborrow semantics.
    match (crate_symbol.as_str(), type_symbol.as_str()) {
        ("bevy_ecs", "Commands") => Some(Reborrowable::Commands),
        ("bevy_ecs", "Deferred") => Some(Reborrowable::Deferred),
        ("bevy_ecs", "DeferredWorld") => Some(Reborrowable::DeferredWorld),
        ("bevy_ecs", "EntityCommands") => Some(Reborrowable::EntityCommands),
        ("bevy_ecs", "EntityMut") => Some(Reborrowable::EntityMut),
        ("bevy_ecs", "FilteredEntityMut") => Some(Reborrowable::FilteredEntityMut),
        ("bevy_ecs", "Mut") => Some(Reborrowable::Mut),
        ("bevy_ecs", "MutUntyped") => Some(Reborrowable::MutUntyped),
        ("bevy_ecs", "NonSendMut") => Some(Reborrowable::NonSendMut),
        ("bevy_ptr", "PtrMut") => Some(Reborrowable::PtrMut),
        ("bevy_ecs", "Query") => Some(Reborrowable::Query),
        ("bevy_ecs", "ResMut") => Some(Reborrowable::ResMut),
        _ => None,
    }
}

/// Suggestion message for deleting a unit value from a bundle tuple.
const REMOVE_UNIT_VALUE: &str = "remove the unit value";

/// Return whether an expression is the unit literal `()`.
pub(crate) const fn is_unit_literal(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Tup([]))
}

/// Collect unit leaves and align them with tuple expressions when possible.
pub(crate) fn collect_unit_bundle_values(
    ty: Ty<'_>,
    expr: &Expr<'_>,
    values: &mut Vec<UnitBundleValue>,
) {
    // Unit values can occur only in tuple-shaped bundle types.
    let ty::Tuple(elements) = ty.kind() else {
        return;
    };
    if elements.is_empty() {
        values.push(UnitBundleValue {
            span: expr.span,
            replacements: Vec::new(),
            suggestion: REMOVE_UNIT_VALUE,
        });
        return;
    }

    // Recurse into aligned tuple literals to retain precise source spans.
    let ExprKind::Tup(expressions) = expr.kind else {
        if tuple_type_contains_unit(ty) {
            values.push(UnitBundleValue {
                span: expr.span,
                replacements: Vec::new(),
                suggestion: REMOVE_UNIT_VALUE,
            });
        }
        return;
    };
    for (index, (element, expression)) in elements.iter().zip(expressions).enumerate() {
        if is_unit_literal(expression) {
            values.push(UnitBundleValue {
                span: expression.span,
                replacements: unit_element_removal(expr, expressions, index),
                suggestion: REMOVE_UNIT_VALUE,
            });
        } else {
            collect_unit_bundle_values(element, expression, values);
        }
    }
}

/// Return replacements that delete one unit element from a tuple literal.
///
/// Deletions never overlap: an element after the first deletes its preceding
/// separator, and the first element deletes the separator after it only when
/// the next element stays.
pub(crate) fn unit_element_removal(
    tuple: &Expr<'_>,
    elements: &[Expr<'_>],
    index: usize,
) -> Vec<(Span, String)> {
    let is_unit = |position: usize| elements.get(position).is_some_and(is_unit_literal);
    // An all-unit tuple has no remaining component to keep.
    if tuple.span.from_expansion() || (0..elements.len()).all(is_unit) {
        return Vec::new();
    }
    let Some(unit) = elements.get(index) else {
        return Vec::new();
    };
    let unit = unit.span;
    // A pair keeps a one-element tuple, which needs a trailing comma.
    let is_pair = elements.len() == 2;
    if let Some(previous) = index
        .checked_sub(1)
        .and_then(|position| elements.get(position))
    {
        let replacement = if is_pair { "," } else { "" };
        return vec![(
            previous.span.between(unit).to(unit),
            String::from(replacement),
        )];
    }
    match elements.get(1) {
        Some(next) if !is_unit(1) => {
            let mut replacements = vec![(unit.until(next.span), String::new())];
            if is_pair {
                replacements.push((next.span.shrink_to_hi(), String::from(",")));
            }
            replacements
        }
        _ => Vec::new(),
    }
}

/// Return whether a nested tuple contains unit.
pub(crate) fn tuple_type_contains_unit(ty: Ty<'_>) -> bool {
    let ty::Tuple(elements) = ty.kind() else {
        return false;
    };
    elements.is_empty() || elements.iter().any(tuple_type_contains_unit)
}

/// Return local trait implementations matching a predicate and their local
/// ADT self types.
pub(crate) fn local_implementations(
    cx: &LateContext<'_>,
    is_trait: impl Fn(DefId) -> bool,
) -> Vec<(DefId, DefId)> {
    // Walk resolved local implementations instead of relying on trait spelling.
    let mut implementations = Vec::new();

    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        if !is_trait(trait_def_id) {
            continue;
        }
        for &impl_def_id in impl_def_ids {
            let self_ty = cx
                .tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip();
            if let ty::Adt(definition, _) = self_ty.kind()
                && definition.did().is_local()
            {
                implementations.push((impl_def_id.to_def_id(), definition.did()));
            }
        }
    }

    implementations
}

/// Return local implementations of one exact trait with their local ADT self types.
pub(crate) fn local_trait_implementations(
    cx: &LateContext<'_>,
    trait_crate: &str,
    trait_name: &str,
) -> Vec<(DefId, DefId)> {
    local_implementations(cx, |def_id| {
        trait_is_named(cx, def_id, trait_crate, trait_name)
    })
}

/// Return each local ADT once from a list of implementations.
pub(crate) fn unique_local_targets(implementations: Vec<(DefId, DefId)>) -> Vec<LocalDefId> {
    let mut targets = Vec::new();
    for (_, self_def_id) in implementations {
        if let Some(local_def_id) = self_def_id.as_local()
            && !targets.contains(&local_def_id)
        {
            targets.push(local_def_id);
        }
    }
    targets
}

/// Return local ADT targets implementing one exact trait.
pub(crate) fn local_trait_targets(
    cx: &LateContext<'_>,
    trait_crate: &str,
    trait_name: &str,
) -> Vec<LocalDefId> {
    unique_local_targets(local_trait_implementations(cx, trait_crate, trait_name))
}

/// Return local ADT targets implementing one standard marker trait.
pub(crate) fn local_standard_trait_targets(
    cx: &LateContext<'_>,
    marker_trait: MarkerTrait,
) -> Vec<LocalDefId> {
    unique_local_targets(local_implementations(cx, |def_id| {
        cx.tcx
            .is_diagnostic_item(marker_trait.diagnostic_item(), def_id)
    }))
}

/// Return whether a local item is a unit struct.
pub(crate) fn local_item_is_unit_struct(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    matches!(
        cx.tcx.hir_expect_item(local_def_id).kind,
        ItemKind::Struct(_, _, VariantData::Unit(..))
    )
}

/// Return whether a local plugin implementation uses default uniqueness.
pub(crate) fn local_plugin_uses_default_uniqueness(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    local_plugin_uniqueness(cx, ty) == Some(LocalPluginUniqueness::DefaultUnique)
}

/// Uniqueness behavior established from one local `Plugin` implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LocalPluginUniqueness {
    /// The implementation inherits Bevy's default unique behavior.
    DefaultUnique,
    /// The implementation overrides `Plugin::is_unique` with an unknown runtime result.
    OverridesIsUnique,
}

/// Return known uniqueness behavior for a local plugin implementation.
///
/// `None` means no local `Plugin` implementation was found for the type.
pub(crate) fn local_plugin_uniqueness(
    cx: &LateContext<'_>,
    ty: Ty<'_>,
) -> Option<LocalPluginUniqueness> {
    // Only local ADTs can have an implementation body available for inspection.
    let ty::Adt(definition, _) = ty.kind() else {
        return None;
    };

    // Inspect only the local implementation for this exact ADT.
    let (impl_def_id, _) = local_trait_implementations(cx, "bevy_app", "Plugin")
        .into_iter()
        .find(|(_, self_def_id)| *self_def_id == definition.did())?;

    // An explicit override may choose either runtime value, so uniqueness is unknown.
    let overrides_uniqueness = cx
        .tcx
        .associated_items(impl_def_id)
        .in_definition_order()
        .any(|item| item.name().as_str() == "is_unique");
    Some(if overrides_uniqueness {
        LocalPluginUniqueness::OverridesIsUnique
    } else {
        LocalPluginUniqueness::DefaultUnique
    })
}

/// Visitor that finds discarded `App::run` expression statements.
pub(crate) struct AppRunVisitor<'a, 'tcx> {
    /// Lint context for method resolution.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// Method spans found in expression-statement position.
    pub(crate) spans: Vec<Span>,
}

impl<'tcx> Visitor<'tcx> for AppRunVisitor<'_, 'tcx> {
    fn visit_stmt(&mut self, statement: &'tcx rustc_hir::Stmt<'tcx>) {
        if let rustc_hir::StmtKind::Semi(expr) = statement.kind
            && let Some(call) = app_method_call(self.cx, expr, "run")
        {
            self.spans.push(call.method_span);
        }
        rustc_hir::intravisit::walk_stmt(self, statement);
    }
}

/// Type visitor that detects one input region in a function output.
pub(crate) struct ContainsRegion<'tcx>(pub(crate) Region<'tcx>);

impl<'tcx> TypeVisitor<TyCtxt<'tcx>> for ContainsRegion<'tcx> {
    type Result = ControlFlow<()>;

    fn visit_region(&mut self, region: Region<'tcx>) -> Self::Result {
        if self.0 == region {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// Query methods that read query data without mutable access.
const READ_ONLY_QUERY_METHODS: &[&str] = &[
    "as_readonly",
    "contains",
    "get",
    "get_many",
    "is_empty",
    "iter",
    "iter_combinations",
    "iter_many",
    "par_iter",
    "single",
];

/// Visitor that classifies one query parameter's method uses.
pub(crate) struct QueryBindingUseVisitor<'a, 'tcx> {
    /// Lint context used to resolve local paths and receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// HIR identifier introduced by the parameter pattern.
    pub(crate) binding_id: rustc_hir::HirId,
    /// Whether a supported read-only query method used the binding.
    pub(crate) saw_read: bool,
    /// Whether any other expression used the binding.
    pub(crate) saw_other_use: bool,
}

impl<'tcx> Visitor<'tcx> for QueryBindingUseVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Classify resolved methods called directly on the tracked query binding.
        if let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind
            && local_path_id(self.cx, receiver) == Some(self.binding_id)
        {
            // The supported method set observes query data without mutable access.
            let is_read = bevy_ecs_method_name(self.cx, expr)
                .is_some_and(|name| READ_ONLY_QUERY_METHODS.contains(&name.as_str()));
            self.saw_read |= is_read;
            self.saw_other_use |= !is_read;
            // Visit arguments while avoiding a second classification of the receiver.
            for argument in arguments {
                self.visit_expr(argument);
            }
            return;
        }
        if local_path_id(self.cx, expr) == Some(self.binding_id) {
            self.saw_other_use = true;
            return;
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that records named field reads on values of selected types.
pub(crate) struct FieldUseVisitor<'a, 'tcx> {
    /// Lint context used to inspect adjusted receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// ADT definitions whose field reads count.
    pub(crate) owners: &'a [DefId],
    /// Unique field names read from the selected types.
    pub(crate) names: Vec<Symbol>,
}

impl<'tcx> Visitor<'tcx> for FieldUseVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Compare the receiver after auto-deref so `Mut<T>` and `&T` count as `T`.
        if let ExprKind::Field(receiver, identifier) = expr.kind
            && let ty::Adt(definition, _) = self
                .cx
                .typeck_results()
                .expr_ty_adjusted(receiver)
                .peel_refs()
                .kind()
            && self.owners.contains(&definition.did())
            && !self.names.contains(&identifier.name)
        {
            self.names.push(identifier.name);
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that distinguishes direct field reads from opaque whole-component uses.
pub(crate) struct ComponentFieldUseVisitor<'a, 'tcx> {
    /// Lint context used to inspect adjusted receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// Local component whose field use is being measured.
    pub(crate) component: LocalDefId,
    /// Unique fields accessed directly.
    pub(crate) names: Vec<Symbol>,
    /// Whether the component escaped direct field access.
    pub(crate) saw_opaque_use: bool,
}

impl<'tcx> Visitor<'tcx> for ComponentFieldUseVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Record direct named-field access without treating its receiver as an escape.
        if let ExprKind::Field(receiver, identifier) = expr.kind
            && local_adt_id(
                self.cx
                    .typeck_results()
                    .expr_ty_adjusted(receiver)
                    .peel_refs(),
            ) == Some(self.component)
        {
            if !self.names.contains(&identifier.name) {
                self.names.push(identifier.name);
            }
            return;
        }

        // Any other expression of the component type consumes the whole value.
        if local_adt_id(self.cx.typeck_results().expr_ty_adjusted(expr).peel_refs())
            == Some(self.component)
        {
            self.saw_opaque_use = true;
            return;
        }

        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that classifies presence-only uses of one query binding.
pub(crate) struct PresenceQueryUseVisitor<'a, 'tcx> {
    /// Lint context used to resolve local paths and receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// HIR identifier introduced by the parameter pattern.
    pub(crate) binding_id: rustc_hir::HirId,
    /// Whether a supported presence-only operation used the binding.
    pub(crate) saw_presence_use: bool,
    /// Whether another expression used the binding.
    pub(crate) saw_other_use: bool,
}

impl<'tcx> Visitor<'tcx> for PresenceQueryUseVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
            if local_path_id(self.cx, expr) == Some(self.binding_id) {
                self.saw_other_use = true;
                return;
            }
            rustc_hir::intravisit::walk_expr(self, expr);
            return;
        };
        // Classify direct methods on the tracked query before visiting nested arguments.
        if local_path_id(self.cx, receiver) == Some(self.binding_id) {
            // `is_empty` observes presence; every other direct method exceeds that scope.
            let is_presence =
                bevy_ecs_method_name(self.cx, expr).is_some_and(|name| name.as_str() == "is_empty");
            self.saw_presence_use |= is_presence;
            self.saw_other_use |= !is_presence;
            for argument in arguments {
                self.visit_expr(argument);
            }
            return;
        }
        // Treat `query.iter().count()` as another presence-only operation.
        let is_iterator_count = self
            .cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .is_some_and(|def_id| {
                self.cx.tcx.item_name(def_id).as_str() == "count"
                    && self
                        .cx
                        .tcx
                        .trait_of_assoc(def_id)
                        .is_some_and(|trait_def_id| {
                            self.cx
                                .tcx
                                .is_diagnostic_item(rustc_span::sym::Iterator, trait_def_id)
                        })
            });
        if is_iterator_count
            && let ExprKind::MethodCall(_, query, [], _) = receiver.kind
            && local_path_id(self.cx, query) == Some(self.binding_id)
            && bevy_ecs_method_name(self.cx, receiver).is_some_and(|name| name.as_str() == "iter")
        {
            self.saw_presence_use = true;
            for argument in arguments {
                self.visit_expr(argument);
            }
            return;
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that classifies one `&mut World` binding's direct uses.
pub(crate) struct WorldBindingUseVisitor<'a, 'tcx> {
    /// Lint context used to resolve local paths and receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// HIR identifier introduced by the parameter pattern.
    pub(crate) binding_id: rustc_hir::HirId,
    /// Whether a narrow typed access used the binding.
    pub(crate) saw_narrow_access: bool,
    /// Whether another expression used the binding.
    pub(crate) saw_other_use: bool,
}

impl<'tcx> Visitor<'tcx> for WorldBindingUseVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
            if local_path_id(self.cx, expr) == Some(self.binding_id) {
                self.saw_other_use = true;
                return;
            }
            rustc_hir::intravisit::walk_expr(self, expr);
            return;
        };
        let cx = self.cx;
        let binding_id = self.binding_id;
        let is_world = |argument: &Expr<'_>| local_path_id(cx, argument) == Some(binding_id);
        // A `QueryState` method that receives the world reads only the query's access.
        if expression_has_type(cx, receiver, "bevy_ecs", "QueryState")
            && arguments.iter().any(is_world)
        {
            self.saw_narrow_access = true;
            self.visit_expr(receiver);
            for argument in arguments.iter().filter(|argument| !is_world(argument)) {
                self.visit_expr(argument);
            }
            return;
        }
        // Classify methods called directly on the tracked world binding.
        if is_world(receiver) {
            // The closed method set exposes narrow resource or entity access.
            let is_narrow = bevy_ecs_method_name(cx, expr).is_some_and(|name| {
                matches!(
                    name.as_str(),
                    "entities"
                        | "get"
                        | "get_mut"
                        | "get_resource"
                        | "get_resource_mut"
                        | "query"
                        | "query_filtered"
                        | "resource"
                        | "resource_mut"
                )
            });
            self.saw_narrow_access |= is_narrow;
            self.saw_other_use |= !is_narrow;
            for argument in arguments {
                self.visit_expr(argument);
            }
            return;
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that separates `Children` reordering from other mutable `Children` uses.
pub(crate) struct ChildrenMutationVisitor<'a, 'tcx> {
    /// Lint context used to resolve methods and adjusted receiver types.
    pub(crate) cx: &'a LateContext<'tcx>,
    /// Whether an inherent `Children` method reordered the entries.
    pub(crate) saw_reorder: bool,
    /// Whether a mutable `Children` handle was used in any other way.
    pub(crate) saw_other_mutation: bool,
}

impl ChildrenMutationVisitor<'_, '_> {
    /// Return whether an expression is a `Mut<Children>` or `&mut Children` handle.
    fn is_mutable_children(&self, expr: &Expr<'_>) -> bool {
        let ty = self.cx.typeck_results().expr_ty(expr);
        if let ty::Ref(_, inner, Mutability::Mut) = ty.kind() {
            return type_is_named(self.cx, *inner, "bevy_ecs", "Children");
        }
        if let ty::Adt(..) = ty.kind()
            && type_is_named(self.cx, ty, "bevy_ecs", "Mut")
        {
            return query_type_arguments(ty)
                .any(|inner| type_is_named(self.cx, inner, "bevy_ecs", "Children"));
        }
        false
    }
}

impl<'tcx> Visitor<'tcx> for ChildrenMutationVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind
            && self.is_mutable_children(receiver)
        {
            // Inherent `Children` methods only reorder; shared receivers only read.
            let is_reorder = self
                .cx
                .typeck_results()
                .type_dependent_def_id(expr.hir_id)
                .and_then(|def_id| self.cx.tcx.inherent_impl_of_assoc(def_id))
                .is_some_and(|impl_def_id| {
                    type_is_named(
                        self.cx,
                        self.cx
                            .tcx
                            .type_of(impl_def_id)
                            .instantiate_identity()
                            .skip_norm_wip(),
                        "bevy_ecs",
                        "Children",
                    )
                });
            let is_shared = matches!(
                self.cx.typeck_results().expr_ty_adjusted(receiver).kind(),
                ty::Ref(_, _, Mutability::Not)
            );
            self.saw_reorder |= is_reorder;
            self.saw_other_mutation |= !is_reorder && !is_shared;
            // Visit the receiver's parts and the arguments without reclassifying the receiver.
            rustc_hir::intravisit::walk_expr(self, receiver);
            for argument in arguments {
                self.visit_expr(argument);
            }
            return;
        }
        if self.is_mutable_children(expr) {
            self.saw_other_mutation = true;
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Return the local binding referenced by a direct path expression.
pub(crate) fn local_path_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<rustc_hir::HirId> {
    // Resolve only direct local paths because projections and calls are distinct uses.
    let ExprKind::Path(ref path) = expr.kind else {
        return None;
    };
    let Res::Local(binding_id) = cx.typeck_results().qpath_res(path, expr.hir_id) else {
        return None;
    };
    Some(binding_id)
}
