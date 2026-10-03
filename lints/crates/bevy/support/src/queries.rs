//! Semantic analysis for Bevy query parameters and component access.

use super::helpers::{
    ChildrenMutationVisitor, ComponentFieldUseVisitor, ContainsRegion, PresenceQueryUseVisitor,
    QueryBindingUseVisitor, WorldBindingUseVisitor, adjustable_inputs,
    collect_shared_reference_replacements, direct_query_refs, is_zst, local_adt_id,
    local_named_fields, local_trait_targets, local_type_size, parameter_binding,
    query_access_width, query_data_has_any_mut_ref, query_data_item_types,
    query_data_mut_components, query_data_type, query_filter_tracked_component, query_filter_type,
    reborrowable_type, used_field_names,
};
use super::{
    Body, ComponentFieldAccess, FnKind, LateContext, LocalDefId, Mutability, Reborrowable, Res,
    Span, TypeVisitable, Visitor, trait_is_named, ty, type_is_named,
};

/// Return parameter indexes containing a mutable query of one managed component.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, local_def_id, component_crate, component_name| {
///     let _ = bevy_support::mutable_query_component_parameters(cx, kind, local_def_id, component_crate, component_name);
/// };
/// ```
pub fn mutable_query_component_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    local_def_id: LocalDefId,
    component_crate: &str,
    component_name: &str,
) -> Vec<usize> {
    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter(|(_, input)| {
                query_data_type(cx, *input).is_some_and(|data| {
                    query_data_mut_components(data).any(|component| {
                        type_is_named(cx, component, component_crate, component_name)
                    })
                })
            })
            .map(|(index, _)| index),
    );
    indexes
}

/// Return mutable `Children` query parameters unless the body only reorders children.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::children_mutation_query_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn children_mutation_query_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let indexes =
        mutable_query_component_parameters(cx, kind, local_def_id, "bevy_ecs", "Children");
    if indexes.is_empty() {
        return indexes;
    }
    // Bevy's inherent `Children` methods reorder entries without breaking the relationship.
    let mut visitor = ChildrenMutationVisitor {
        cx,
        saw_reorder: false,
        saw_other_mutation: false,
    };
    visitor.visit_expr(body.value);
    if visitor.saw_reorder && !visitor.saw_other_mutation {
        return Vec::new();
    }
    indexes
}

/// Return parameter indexes that borrow a reborrowable Bevy proxy.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, local_def_id| {
///     let _ = bevy_support::borrowed_reborrowable_parameters(cx, kind, local_def_id);
/// };
/// ```
pub fn borrowed_reborrowable_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<(usize, Reborrowable)> {
    let inputs = adjustable_inputs(cx, kind, local_def_id);
    // The output type decides whether a borrowed region must outlive the call.
    let output = match kind {
        FnKind::Closure => cx.tcx.closure_user_provided_sig(local_def_id).value,
        FnKind::ItemFn(..) | FnKind::Method(..) => cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    }
    .skip_binder()
    .output();

    let mut parameters = Vec::new();
    parameters.extend(inputs.into_iter().filter_map(|(index, input)| {
        let ty::Ref(region, inner, Mutability::Mut) = input.kind() else {
            return None;
        };
        // A returned borrow of the proxy needs the outer reference.
        if output.visit_with(&mut ContainsRegion(*region)).is_break() {
            return None;
        }
        reborrowable_type(cx, *inner).map(|reborrowable| (index, reborrowable))
    }));
    parameters
}

/// Return parameter indexes whose query data contains a direct ZST reference.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, local_def_id| {
///     let _ = bevy_support::zst_query_parameters(cx, kind, local_def_id);
/// };
/// ```
pub fn zst_query_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter(|(_, input)| {
                query_data_type(cx, *input).is_some_and(|query_data| {
                    direct_query_refs(query_data).any(|ty| is_zst(cx, ty))
                })
            })
            .map(|(index, _)| index),
    );
    indexes
}

/// Return mutable query parameters used only through read-only query methods.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::readonly_mut_query_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn readonly_mut_query_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter_map(|(index, input)| {
                let query_data = query_data_type(cx, input)?;
                if !query_data_has_any_mut_ref(query_data) {
                    return None;
                }
                let binding_id = parameter_binding(body, index)?;
                // Every direct use must be one of the read-only query methods.
                let mut visitor = QueryBindingUseVisitor {
                    cx,
                    binding_id,
                    saw_read: false,
                    saw_other_use: false,
                };
                visitor.visit_expr(body.value);
                (visitor.saw_read && !visitor.saw_other_use).then_some(index)
            }),
    );
    indexes
}

/// Return replacements that turn mutable `Query` references
/// into shared references.
///
/// The result is empty unless the declared type spells `Query<D, ..>` directly
/// and `D` is a mutable reference or tuple of direct mutable references.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, parameter| {
///     let _ = bevy_support::shared_query_data_replacements(cx, parameter);
/// };
/// ```
pub fn shared_query_data_replacements(
    cx: &LateContext<'_>,
    parameter: &rustc_hir::Ty<'_>,
) -> Vec<(Span, String)> {
    // Peel a reference to the query, then require a resolved `Query` path.
    let query = if let rustc_hir::TyKind::Ref(_, mutable) = parameter.kind {
        mutable.ty
    } else {
        parameter
    };
    let rustc_hir::TyKind::Path(rustc_hir::QPath::Resolved(None, path)) = query.kind else {
        return Vec::new();
    };
    let Res::Def(_, def_id) = path.res else {
        return Vec::new();
    };
    if !trait_is_named(cx, def_id, "bevy_ecs", "Query") || parameter.span.from_expansion() {
        return Vec::new();
    }
    let Some(data) = path
        .segments
        .last()
        .and_then(|segment| segment.args)
        .and_then(|arguments| {
            arguments.args.iter().find_map(|argument| {
                if let rustc_hir::GenericArg::Type(ty) = argument {
                    Some(ty.as_unambig_ty())
                } else {
                    None
                }
            })
        })
    else {
        return Vec::new();
    };
    // Rewrite each `&mut` prefix while keeping any explicit lifetime.
    let mut replacements = Vec::new();
    collect_shared_reference_replacements(cx, data, &mut replacements);
    replacements
}

/// Return query parameters that fetch whole entities before fixed typed access.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::unfiltered_entity_access_query_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn unfiltered_entity_access_query_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let mut indexes = Vec::new();
    for (index, input) in adjustable_inputs(cx, kind, local_def_id) {
        let has_entity_proxy = query_data_type(cx, input).is_some_and(|data| {
            type_is_named(cx, data, "bevy_ecs", "EntityRef")
                || type_is_named(cx, data, "bevy_ecs", "EntityMut")
        });
        let has_query_origin = parameter_binding(body, index).is_some_and(|query_binding| {
            super::query_origins::body_contains_fixed_entity_access(cx, body, query_binding)
        });
        if has_entity_proxy && has_query_origin {
            indexes.push(index);
        }
    }
    indexes
}

/// Return local non-resource components with at least eight fields and a
/// layout over 64 bytes.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::local_large_components(cx);
/// };
/// ```
pub fn local_large_components(cx: &LateContext<'_>) -> impl Iterator<Item = LocalDefId> {
    // Bevy 0.19 implements `Component` for resources as an internal storage detail.
    let resources = local_trait_targets(cx, "bevy_ecs", "Resource");

    local_trait_targets(cx, "bevy_ecs", "Component")
        .into_iter()
        .filter(move |target| !resources.contains(target))
        .filter(|target| local_named_fields(cx, *target).len() >= 8)
        .filter(|target| local_type_size(cx, *target).is_some_and(|bytes| bytes > 64))
}

/// Return query parameters whose component access exceeds the supported width.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, local_def_id| {
///     let _ = bevy_support::wide_query_parameters(cx, kind, local_def_id);
/// };
/// ```
pub fn wide_query_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let query_data_targets = local_trait_targets(cx, "bevy_ecs", "QueryData");

    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter_map(|(index, input)| {
                let data = query_data_type(cx, input)?;
                let (total, mutable) = query_access_width(cx, data, &query_data_targets);
                // A named local `QueryData` type groups related access, so it gets a higher total limit.
                let is_custom_query_data =
                    local_adt_id(data).is_some_and(|target| query_data_targets.contains(&target));
                let total_limit = if is_custom_query_data { 8 } else { 5 };
                (total > total_limit || mutable > 4).then_some(index)
            }),
    );
    indexes
}

/// Return broad custom query parameters that use at most half their named fields.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::partially_used_query_data_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn partially_used_query_data_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let query_data_targets = local_trait_targets(cx, "bevy_ecs", "QueryData");

    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter_map(|(index, input)| {
                let data = query_data_type(cx, input)?;
                let target = local_adt_id(data)?;
                if !query_data_targets.contains(&target) {
                    return None;
                }
                let fields = local_named_fields(cx, target);
                if fields.len() <= 8 {
                    return None;
                }
                // Count fields read through the derived item structs of this query data type.
                let owners = query_data_item_types(cx, target);
                let used = used_field_names(cx, body, &owners)
                    .iter()
                    .filter(|field| fields.contains(field))
                    .count();
                (used * 2 <= fields.len()).then_some(index)
            }),
    );
    indexes
}

/// Return shared query parameters used only for presence or count checks.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::presence_only_query_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn presence_only_query_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter_map(|(index, input)| {
                let data = query_data_type(cx, input)?;
                if !matches!(data.kind(), ty::Ref(_, _, Mutability::Not)) {
                    return None;
                }
                let binding_id = parameter_binding(body, index)?;
                let mut visitor = PresenceQueryUseVisitor {
                    cx,
                    binding_id,
                    saw_presence_use: false,
                    saw_other_use: false,
                };
                visitor.visit_expr(body.value);
                (visitor.saw_presence_use && !visitor.saw_other_use).then_some(index)
            }),
    );
    indexes
}

/// Return a narrow exclusive-system parameter that can use normal system parameters.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::narrow_exclusive_system_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn narrow_exclusive_system_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    if !matches!(kind, FnKind::ItemFn(..)) {
        return Vec::new();
    }

    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter_map(|(index, input)| {
                let ty::Ref(_, inner, Mutability::Mut) = input.kind() else {
                    return None;
                };
                if !type_is_named(cx, *inner, "bevy_ecs", "World") {
                    return None;
                }
                let binding_id = parameter_binding(body, index)?;
                let mut visitor = WorldBindingUseVisitor {
                    cx,
                    binding_id,
                    saw_narrow_access: false,
                    saw_other_use: false,
                };
                visitor.visit_expr(body.value);
                (visitor.saw_narrow_access && !visitor.saw_other_use).then_some(index)
            }),
    );
    indexes
}

/// Return query parameters that change-track a large local component.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::large_component_change_filter_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn large_component_change_filter_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Vec<usize> {
    let inputs = adjustable_inputs(cx, kind, local_def_id);
    let large_components = local_large_components(cx).collect::<Vec<_>>();

    let mut indexes = Vec::new();
    indexes.extend(inputs.into_iter().filter_map(|(index, input)| {
        let filter = query_filter_type(cx, input)?;
        let component = query_filter_tracked_component(cx, filter, &large_components)?;
        // Count only fields read from values of the tracked component type.
        let fields = local_named_fields(cx, component);
        let used = used_field_names(cx, body, &[component.to_def_id()]).len();
        (used * 2 < fields.len()).then_some(index)
    }));
    indexes
}

/// Return field access for a function with one mutable local component query.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::mutable_component_field_access(cx, kind, body, local_def_id);
/// };
/// ```
pub fn mutable_component_field_access<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> Option<ComponentFieldAccess> {
    // Limit this analysis to named function items with a stable signature.
    if !matches!(kind, FnKind::ItemFn(..)) {
        return None;
    }
    let components = adjustable_inputs(cx, kind, local_def_id)
        .into_iter()
        .filter_map(|(_, input)| query_data_type(cx, input))
        .flat_map(query_data_mut_components)
        .filter_map(local_adt_id)
        .collect::<Vec<_>>();
    let [component] = components.as_slice() else {
        return None;
    };
    // Compare body accesses with the component's complete named-field set.
    let component_fields = local_named_fields(cx, *component);
    let mut visitor = ComponentFieldUseVisitor {
        cx,
        component: *component,
        names: Vec::new(),
        saw_opaque_use: false,
    };
    visitor.visit_expr(body.value);
    // Whole-value uses prevent a safe field-level projection.
    if visitor.saw_opaque_use {
        return None;
    }
    let fields = component_fields
        .into_iter()
        .filter(|field| visitor.names.contains(field))
        .collect::<Vec<_>>();
    (!fields.is_empty()).then_some(ComponentFieldAccess {
        component: *component,
        fields,
    })
}
