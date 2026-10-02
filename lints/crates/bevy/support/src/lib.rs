#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for Bevy-specific private lints.

extern crate rustc_abi;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use core::ops::ControlFlow;

use rustc_abi::Size;
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl, ItemKind, Mutability, PatKind, VariantData,
    def::Res,
    def_id::{DefId, LocalDefId},
    intravisit::{FnKind, Visitor},
};
use rustc_lint::LateContext;
use rustc_middle::ty::{
    self, Region, Ty, TyCtxt, TypeVisitable, TypeVisitor,
    layout::{LayoutOf, TyAndLayout},
};
use rustc_span::{Span, Symbol};

use dylint_linting as _;
use dylint_support as _;

/// One semantically resolved Bevy method call.
#[derive(Clone, Copy, Debug)]
pub struct BevyMethodCall<'hir> {
    /// Receiver expression.
    pub receiver: &'hir Expr<'hir>,
    /// Explicit arguments, excluding the receiver.
    pub arguments: &'hir [Expr<'hir>],
    /// Span of the method identifier.
    pub method_span: Span,
    /// Resolved method definition.
    pub def_id: DefId,
}

/// Field-level access performed through one mutable component query.
#[derive(Clone, Debug)]
pub struct ComponentFieldAccess {
    /// Local component definition.
    pub component: LocalDefId,
    /// Named component fields accessed by the function.
    pub fields: Vec<Symbol>,
}

/// Direct free systems registered by one `App::add_systems` call.
#[derive(Clone, Debug)]
pub struct RegisteredSystems {
    /// Resolved schedule-label type name.
    pub schedule: Symbol,
    /// Direct local free-function definitions.
    pub systems: Vec<LocalDefId>,
}

/// A standard trait expected on an empty marker component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkerTrait {
    /// The `Clone` trait.
    Clone,
    /// The `Copy` trait.
    Copy,
    /// The `Default` trait.
    Default,
}

impl MarkerTrait {
    /// Return the standard trait name.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = bevy_support::MarkerTrait::name(value);
    /// };
    /// ```
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clone => "Clone",
            Self::Copy => "Copy",
            Self::Default => "Default",
        }
    }

    /// Return the trait's rustc diagnostic item.
    const fn diagnostic_item(self) -> Symbol {
        match self {
            Self::Clone => rustc_span::sym::Clone,
            Self::Copy => rustc_span::sym::Copy,
            Self::Default => rustc_span::sym::Default,
        }
    }
}

/// A Bevy proxy type whose value can be cheaply reborrowed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reborrowable {
    /// `Commands`.
    Commands,
    /// `Deferred`.
    Deferred,
    /// `DeferredWorld`.
    DeferredWorld,
    /// `EntityCommands`.
    EntityCommands,
    /// `EntityMut`.
    EntityMut,
    /// `FilteredEntityMut`.
    FilteredEntityMut,
    /// Bevy's change-detecting `Mut`.
    Mut,
    /// Bevy's type-erased `MutUntyped`.
    MutUntyped,
    /// `NonSendMut`.
    NonSendMut,
    /// `PtrMut`.
    PtrMut,
    /// `Query`.
    Query,
    /// `ResMut`.
    ResMut,
}

impl Reborrowable {
    /// Return the user-facing type name.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = bevy_support::Reborrowable::name(value);
    /// };
    /// ```
    pub const fn name(self) -> &'static str {
        match self {
            Self::Commands => "Commands",
            Self::Deferred => "Deferred",
            Self::DeferredWorld => "DeferredWorld",
            Self::EntityCommands => "EntityCommands",
            Self::EntityMut => "EntityMut",
            Self::FilteredEntityMut => "FilteredEntityMut",
            Self::Mut => "Mut",
            Self::MutUntyped => "MutUntyped",
            Self::NonSendMut => "NonSendMut",
            Self::PtrMut => "PtrMut",
            Self::Query => "Query",
            Self::ResMut => "ResMut",
        }
    }
}

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
    if !body_contains_fixed_entity_access(cx, body) {
        return Vec::new();
    }

    let mut indexes = Vec::new();
    indexes.extend(
        adjustable_inputs(cx, kind, local_def_id)
            .into_iter()
            .filter(|(_, input)| {
                query_data_type(cx, *input).is_some_and(|data| {
                    type_is_named(cx, data, "bevy_ecs", "EntityRef")
                        || type_is_named(cx, data, "bevy_ecs", "EntityMut")
                })
            })
            .map(|(index, _)| index),
    );
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

/// Return local component-like types that lack `Reflect`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::local_bevy_types_missing_reflect(cx);
/// };
/// ```
pub fn local_bevy_types_missing_reflect(cx: &LateContext<'_>) -> Vec<LocalDefId> {
    // Build the exclusion set once before scanning each Bevy marker trait.
    let reflected = local_trait_targets(cx, "bevy_reflect", "Reflect");
    let mut targets = Vec::new();

    for (crate_name, trait_name) in [
        ("bevy_ecs", "Component"),
        ("bevy_ecs", "Resource"),
        ("bevy_ecs", "Message"),
        ("bevy_ecs", "Event"),
    ] {
        // Deduplicate types that implement more than one Bevy marker trait.
        for target in local_trait_targets(cx, crate_name, trait_name) {
            if reflected.contains(&target) || targets.contains(&target) {
                continue;
            }
            targets.push(target);
        }
    }

    targets
}

/// Return local unit components missing one standard trait.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, marker_trait| {
///     let _ = bevy_support::local_unit_components_missing_trait(cx, marker_trait);
/// };
/// ```
pub fn local_unit_components_missing_trait(
    cx: &LateContext<'_>,
    marker_trait: MarkerTrait,
) -> impl Iterator<Item = LocalDefId> {
    let implemented = local_standard_trait_targets(cx, marker_trait);

    local_trait_targets(cx, "bevy_ecs", "Component")
        .into_iter()
        .filter(|target| local_item_is_unit_struct(cx, *target))
        .filter(move |target| !implemented.contains(target))
}

/// Return an attribute insertion that derives a trait on a unit component.
///
/// The result is `None` when the derive could fail to compile: when the item
/// comes from a macro expansion, or when `Copy` is missing its `Clone` supertrait.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, target, marker_trait| {
///     let _ = bevy_support::missing_trait_derive(cx, target, marker_trait);
/// };
/// ```
pub fn missing_trait_derive(
    cx: &LateContext<'_>,
    target: LocalDefId,
    marker_trait: MarkerTrait,
) -> Option<(Span, String)> {
    let span = cx.tcx.def_span(target);
    if span.from_expansion() {
        return None;
    }
    // `Copy` requires `Clone`, so its derive compiles only next to an existing `Clone`.
    if marker_trait == MarkerTrait::Copy
        && !local_standard_trait_targets(cx, MarkerTrait::Clone).contains(&target)
    {
        return None;
    }
    Some((
        span.shrink_to_lo(),
        format!("#[derive({})]\n", marker_trait.name()),
    ))
}

/// Return local Bevy trait implementations whose names violate conventions.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::unconventional_bevy_type_names(cx);
/// };
/// ```
pub fn unconventional_bevy_type_names(
    cx: &LateContext<'_>,
) -> Vec<(LocalDefId, &'static str, &'static str)> {
    // Accumulate one violation per local type across the supported Bevy traits.
    let mut violations = Vec::new();

    for (trait_name, suffix) in [("Plugin", "Plugin"), ("SystemSet", "Systems")] {
        // Search both crates that expose the naming-constrained traits.
        for target in local_trait_targets(cx, "bevy_app", trait_name)
            .into_iter()
            .chain(local_trait_targets(cx, "bevy_ecs", trait_name))
        {
            let name = cx.tcx.item_name(target.to_def_id());
            let has_expected_suffix = name.as_str().ends_with(suffix);
            if has_expected_suffix {
                continue;
            }
            // A type implementing both resolved traits still needs one diagnostic.
            let is_already_reported = violations
                .iter()
                .any(|(existing, _, _)| *existing == target);
            if !is_already_reported {
                violations.push((target, trait_name, suffix));
            }
        }
    }

    violations
}

/// Count direct dependencies whose original crate name is `bevy`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::direct_bevy_facades(cx);
/// };
/// ```
pub fn direct_bevy_facades(cx: &LateContext<'_>) -> usize {
    cx.tcx
        .crates(())
        .iter()
        .filter(|&&crate_num| {
            cx.tcx.crate_name(crate_num).as_str() == "bevy"
                && cx
                    .tcx
                    .extern_crate(crate_num)
                    .is_some_and(rustc_session::cstore::ExternCrate::is_direct)
        })
        .count()
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

/// Declare one panicking `World` method lint.
#[macro_export]
macro_rules! declare_world_method_lint {
    (
        $lint:ident, $pass:ident, $level:ident, $method:literal, $alternative:literal,
        $description:literal, $message:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            $level,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check an exact panicking method on `bevy_ecs::world::World`.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                let Some(call) = $crate::world_method_call(cx, expr, $method) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.method_span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help(concat!(
                                "use `World::",
                                $alternative,
                                "` and handle the fallible result"
                            ));
                    }),
                );
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint whose semantic matcher returns one expression span.
#[macro_export]
macro_rules! declare_expression_span_lint {
    (
        $lint:ident, $pass:ident, $level:ident, $checker:path, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            $level,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check one semantically resolved Bevy expression.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                let Some(span) = $checker(cx, expr) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint against mutable query access to one managed Bevy component.
#[macro_export]
macro_rules! declare_mutable_query_component_lint {
    (
        $lint:ident, $pass:ident, $component_crate:literal, $component:literal,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check function parameters for mutable query access to the managed component.
            fn check_fn(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                kind: rustc_hir::intravisit::FnKind<'tcx>,
                declaration: &'tcx rustc_hir::FnDecl<'tcx>,
                _: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                let indexes = $crate::mutable_query_component_parameters(
                    cx,
                    kind,
                    local_def_id,
                    $component_crate,
                    $component,
                );
                for span in $crate::parameter_spans(declaration, indexes) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic =
                                diagnostic.primary_message($message).help($help);
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint whose matcher returns function-parameter indexes.
#[macro_export]
macro_rules! declare_function_parameter_lint {
    (
        $lint:ident, $pass:ident, $checker:path, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check function parameters through the configured semantic matcher.
            fn check_fn(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                kind: rustc_hir::intravisit::FnKind<'tcx>,
                declaration: &'tcx rustc_hir::FnDecl<'tcx>,
                body: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                let indexes = $checker(cx, kind, body, local_def_id);
                for span in $crate::parameter_spans(declaration, indexes) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic =
                                diagnostic.primary_message($message).help($help);
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint whose type-only matcher returns function-parameter indexes.
#[macro_export]
macro_rules! declare_function_parameter_type_lint {
    (
        $lint:ident, $pass:ident, $checker:path, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check function parameter types through the configured semantic matcher.
            fn check_fn(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                kind: rustc_hir::intravisit::FnKind<'tcx>,
                declaration: &'tcx rustc_hir::FnDecl<'tcx>,
                _: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                let indexes = $checker(cx, kind, local_def_id);
                for span in $crate::parameter_spans(declaration, indexes) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic =
                                diagnostic.primary_message($message).help($help);
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare one project schedule policy lint.
#[macro_export]
macro_rules! declare_disallowed_schedule_lint {
    (
        $lint:ident, $pass:ident, $schedule:literal, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check `App::add_systems` for the configured schedule label.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                let Some(span) = $crate::disallowed_schedule_span(cx, expr, $schedule) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare one trait convention lint for unit components.
#[macro_export]
macro_rules! declare_missing_unit_component_trait_lint {
    (
        $lint:ident, $pass:ident, $marker_trait:ident, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check local unit components for the configured standard trait.
            fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
                use rustc_lint::LintContext as _;

                let marker_trait = $crate::MarkerTrait::$marker_trait;
                for target in $crate::local_unit_components_missing_trait(cx, marker_trait) {
                    let derive = $crate::missing_trait_derive(cx, target, marker_trait);
                    cx.emit_span_lint(
                        $lint,
                        cx.tcx.def_span(target),
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic = diagnostic.primary_message($message);
                            // Offer the derive only where it is known to compile.
                            if let Some((span, attribute)) = derive {
                                let _suggested = diagnostic.span_suggestion_verbose(
                                    span,
                                    $help,
                                    attribute,
                                    rustc_errors::Applicability::MachineApplicable,
                                );
                            } else {
                                let _helped = diagnostic.help($help);
                            }
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Return indexed parameter types for a function whose signature
/// the author controls.
///
/// Trait implementation methods yield nothing because traits fix their
/// parameter types.
fn adjustable_inputs<'tcx>(
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
fn closure_body<'tcx>(
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
fn query_data_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    type_is_named(cx, ty, "bevy_ecs", "Query")
        .then(|| query_type_arguments(ty).next())
        .flatten()
}

/// Return query filters from an instantiated `Query` type.
fn query_filter_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    type_is_named(cx, ty, "bevy_ecs", "Query")
        .then(|| query_type_arguments(ty).nth(1))
        .flatten()
}

/// Return the type arguments of an ADT after peeling references.
fn query_type_arguments(ty: Ty<'_>) -> impl Iterator<Item = Ty<'_>> {
    let arguments = if let ty::Adt(_, arguments) = ty.peel_refs().kind() {
        arguments.types().collect()
    } else {
        Vec::new()
    };
    arguments.into_iter()
}

/// Return the binding introduced by a plain identifier parameter.
fn parameter_binding(body: &Body<'_>, index: usize) -> Option<rustc_hir::HirId> {
    let parameter = body.params.get(index)?;
    let PatKind::Binding(_, binding_id, _, None) = parameter.pat.kind else {
        return None;
    };
    Some(binding_id)
}

/// Collect `&mut` prefix replacements from HIR query data made of references and tuples.
fn collect_shared_reference_replacements(
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
fn direct_query_refs(ty: Ty<'_>) -> impl Iterator<Item = Ty<'_>> {
    let mut references = Vec::new();
    collect_direct_query_refs(ty, &mut references);
    references.into_iter()
}

/// Collect direct shared or mutable reference leaves from tuple query data.
fn collect_direct_query_refs<'tcx>(ty: Ty<'tcx>, references: &mut Vec<Ty<'tcx>>) {
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
fn query_data_mut_components(ty: Ty<'_>) -> impl Iterator<Item = Ty<'_>> {
    let mut components = Vec::new();
    collect_query_data_mut_components(ty, &mut components);
    components.into_iter()
}

/// Collect mutable reference leaves from tuple query data.
fn collect_query_data_mut_components<'tcx>(ty: Ty<'tcx>, components: &mut Vec<Ty<'tcx>>) {
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
fn query_data_has_any_mut_ref(ty: Ty<'_>) -> bool {
    query_data_mut_components(ty).next().is_some()
}

/// Return whether a query filter includes `With<Camera>`.
fn query_filter_has_camera(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
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
fn local_adt_id(ty: Ty<'_>) -> Option<LocalDefId> {
    let ty::Adt(definition, _) = ty.peel_refs().kind() else {
        return None;
    };
    definition.did().as_local()
}

/// Return the names of fields on a local named-field struct.
fn local_named_fields(cx: &LateContext<'_>, local_def_id: LocalDefId) -> Vec<Symbol> {
    let ItemKind::Struct(_, _, VariantData::Struct { fields, .. }) =
        cx.tcx.hir_expect_item(local_def_id).kind
    else {
        return Vec::new();
    };
    fields.iter().map(|field| field.ident.name).collect()
}

/// Return the layout size of one monomorphic local type.
fn local_type_size(cx: &LateContext<'_>, local_def_id: LocalDefId) -> Option<u64> {
    let ty = cx
        .tcx
        .type_of(local_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    cx.layout_of(ty).ok().map(|layout| layout.size.bytes())
}

/// Count all and mutable component accesses in query data.
fn query_access_width<'tcx>(
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
fn query_filter_tracked_component(
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
fn query_data_item_types(cx: &LateContext<'_>, target: LocalDefId) -> Vec<DefId> {
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
fn used_field_names<'tcx>(
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

/// Return whether a body performs a fixed typed access through an entity view.
fn body_contains_fixed_entity_access<'tcx>(cx: &LateContext<'tcx>, body: &Body<'tcx>) -> bool {
    let mut visitor = FixedEntityAccessVisitor {
        cx,
        is_found: false,
    };
    visitor.visit_expr(body.value);
    visitor.is_found
}

/// Return the system expressions inside an `add_systems` argument.
///
/// Tuples are flattened, and schedule configuration methods such as `run_if`,
/// `chain`, or `after` are looked through to the systems they configure.
fn system_expressions<'hir>(cx: &LateContext<'_>, expr: &'hir Expr<'hir>) -> Vec<&'hir Expr<'hir>> {
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
fn system_function(cx: &LateContext<'_>, system: &Expr<'_>) -> Option<DefId> {
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
fn app_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    method_name: &str,
) -> Option<BevyMethodCall<'hir>> {
    bevy_method_call(cx, expr, "bevy_app", "App", method_name)
        .or_else(|| bevy_method_call(cx, expr, "bevy_app", "SubApp", method_name))
}

/// Return an ADT's item name.
fn adt_name(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Symbol> {
    let ty::Adt(definition, _) = ty.kind() else {
        return None;
    };
    Some(cx.tcx.item_name(definition.did()))
}

/// Return the name of a method call resolved to a `bevy_ecs` definition.
fn bevy_ecs_method_name(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Symbol> {
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    (cx.tcx.crate_name(def_id.krate).as_str() == "bevy_ecs").then(|| cx.tcx.item_name(def_id))
}

/// Return whether a callee path resolves to a method of one diagnostic-item trait.
fn resolved_trait_method(cx: &LateContext<'_>, callee: &Expr<'_>, trait_item: Symbol) -> bool {
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
fn is_zst<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(
        cx.layout_of(ty),
        Ok(TyAndLayout { layout, .. }) if layout.size() == Size::ZERO
    )
}

/// Recognize one supported reborrowable proxy.
fn reborrowable_type(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Reborrowable> {
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
const fn is_unit_literal(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Tup([]))
}

/// Collect unit leaves and align them with tuple expressions when possible.
fn collect_unit_bundle_values(ty: Ty<'_>, expr: &Expr<'_>, values: &mut Vec<UnitBundleValue>) {
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
fn unit_element_removal(
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
fn tuple_type_contains_unit(ty: Ty<'_>) -> bool {
    let ty::Tuple(elements) = ty.kind() else {
        return false;
    };
    elements.is_empty() || elements.iter().any(tuple_type_contains_unit)
}

/// Return local trait implementations matching a predicate and their local
/// ADT self types.
fn local_implementations(
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
fn local_trait_implementations(
    cx: &LateContext<'_>,
    trait_crate: &str,
    trait_name: &str,
) -> Vec<(DefId, DefId)> {
    local_implementations(cx, |def_id| {
        trait_is_named(cx, def_id, trait_crate, trait_name)
    })
}

/// Return each local ADT once from a list of implementations.
fn unique_local_targets(implementations: Vec<(DefId, DefId)>) -> Vec<LocalDefId> {
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
fn local_trait_targets(
    cx: &LateContext<'_>,
    trait_crate: &str,
    trait_name: &str,
) -> Vec<LocalDefId> {
    unique_local_targets(local_trait_implementations(cx, trait_crate, trait_name))
}

/// Return local ADT targets implementing one standard marker trait.
fn local_standard_trait_targets(
    cx: &LateContext<'_>,
    marker_trait: MarkerTrait,
) -> Vec<LocalDefId> {
    unique_local_targets(local_implementations(cx, |def_id| {
        cx.tcx
            .is_diagnostic_item(marker_trait.diagnostic_item(), def_id)
    }))
}

/// Return whether a local item is a unit struct.
fn local_item_is_unit_struct(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    matches!(
        cx.tcx.hir_expect_item(local_def_id).kind,
        ItemKind::Struct(_, _, VariantData::Unit(..))
    )
}

/// Return whether a local plugin implementation uses `Plugin::is_unique`.
fn local_plugin_uses_default_uniqueness(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    // Only local ADTs can have an implementation body available for inspection.
    let ty::Adt(definition, _) = ty.kind() else {
        return false;
    };

    // Any explicit `is_unique` method replaces the trait's default behavior.
    local_trait_implementations(cx, "bevy_app", "Plugin")
        .into_iter()
        .find(|(_, self_def_id)| *self_def_id == definition.did())
        .is_some_and(|(impl_def_id, _)| {
            !cx.tcx
                .associated_items(impl_def_id)
                .in_definition_order()
                .any(|item| item.name().as_str() == "is_unique")
        })
}

/// Visitor that finds discarded `App::run` expression statements.
struct AppRunVisitor<'a, 'tcx> {
    /// Lint context for method resolution.
    cx: &'a LateContext<'tcx>,
    /// Method spans found in expression-statement position.
    spans: Vec<Span>,
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
struct ContainsRegion<'tcx>(Region<'tcx>);

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
struct QueryBindingUseVisitor<'a, 'tcx> {
    /// Lint context used to resolve local paths and receiver types.
    cx: &'a LateContext<'tcx>,
    /// HIR identifier introduced by the parameter pattern.
    binding_id: rustc_hir::HirId,
    /// Whether a supported read-only query method used the binding.
    saw_read: bool,
    /// Whether any other expression used the binding.
    saw_other_use: bool,
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

/// Visitor that finds fixed typed access through `EntityRef` or `EntityMut`.
struct FixedEntityAccessVisitor<'a, 'tcx> {
    /// Lint context used to resolve receiver types.
    cx: &'a LateContext<'tcx>,
    /// Whether a supported access was found.
    is_found: bool,
}

impl<'tcx> Visitor<'tcx> for FixedEntityAccessVisitor<'_, 'tcx> {
    fn visit_nested_body(&mut self, body_id: rustc_hir::BodyId) {
        if let Some(body) = closure_body(self.cx, body_id) {
            self.visit_body(body);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Match only fixed typed access methods on the two supported entity proxies.
        if let ExprKind::MethodCall(_, receiver, _, _) = expr.kind
            && (expression_has_type(self.cx, receiver, "bevy_ecs", "EntityRef")
                || expression_has_type(self.cx, receiver, "bevy_ecs", "EntityMut"))
            && bevy_ecs_method_name(self.cx, expr).is_some_and(|name| {
                matches!(
                    name.as_str(),
                    "contains"
                        | "get"
                        | "get_components"
                        | "get_components_mut"
                        | "get_mut"
                        | "get_ref"
                )
            })
        {
            self.is_found = true;
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that records named field reads on values of selected types.
struct FieldUseVisitor<'a, 'tcx> {
    /// Lint context used to inspect adjusted receiver types.
    cx: &'a LateContext<'tcx>,
    /// ADT definitions whose field reads count.
    owners: &'a [DefId],
    /// Unique field names read from the selected types.
    names: Vec<Symbol>,
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
struct ComponentFieldUseVisitor<'a, 'tcx> {
    /// Lint context used to inspect adjusted receiver types.
    cx: &'a LateContext<'tcx>,
    /// Local component whose field use is being measured.
    component: LocalDefId,
    /// Unique fields accessed directly.
    names: Vec<Symbol>,
    /// Whether the component escaped direct field access.
    saw_opaque_use: bool,
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
struct PresenceQueryUseVisitor<'a, 'tcx> {
    /// Lint context used to resolve local paths and receiver types.
    cx: &'a LateContext<'tcx>,
    /// HIR identifier introduced by the parameter pattern.
    binding_id: rustc_hir::HirId,
    /// Whether a supported presence-only operation used the binding.
    saw_presence_use: bool,
    /// Whether another expression used the binding.
    saw_other_use: bool,
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
struct WorldBindingUseVisitor<'a, 'tcx> {
    /// Lint context used to resolve local paths and receiver types.
    cx: &'a LateContext<'tcx>,
    /// HIR identifier introduced by the parameter pattern.
    binding_id: rustc_hir::HirId,
    /// Whether a narrow typed access used the binding.
    saw_narrow_access: bool,
    /// Whether another expression used the binding.
    saw_other_use: bool,
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
struct ChildrenMutationVisitor<'a, 'tcx> {
    /// Lint context used to resolve methods and adjusted receiver types.
    cx: &'a LateContext<'tcx>,
    /// Whether an inherent `Children` method reordered the entries.
    saw_reorder: bool,
    /// Whether a mutable `Children` handle was used in any other way.
    saw_other_mutation: bool,
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
fn local_path_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<rustc_hir::HirId> {
    // Resolve only direct local paths because projections and calls are distinct uses.
    let ExprKind::Path(ref path) = expr.kind else {
        return None;
    };
    let Res::Local(binding_id) = cx.typeck_results().qpath_res(path, expr.hir_id) else {
        return None;
    };
    Some(binding_id)
}
