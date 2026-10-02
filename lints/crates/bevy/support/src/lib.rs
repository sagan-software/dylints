#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for Bevy-specific private lints.

extern crate rustc_abi;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
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
/// let _call = |cx, kind, body, local_def_id, component_crate, component_name| {
///     let _ = bevy_support::mutable_query_component_parameters(cx, kind, body, local_def_id, component_crate, component_name);
/// };
/// ```
pub fn mutable_query_component_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
    component_crate: &str,
    component_name: &str,
) -> impl Iterator<Item = usize> {
    let signature = function_signature(cx, kind, local_def_id);

    signature
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let is_self = body.params.get(index).is_some_and(|parameter| {
                matches!(
                    parameter.pat.kind,
                    PatKind::Binding(_, _, identifier, _) if identifier.name.as_str() == "self"
                )
            });
            (!is_self && query_data_has_mut_component(cx, *input, component_crate, component_name))
                .then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// Return parameter indexes that borrow a reborrowable Bevy proxy.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, body, local_def_id| {
///     let _ = bevy_support::borrowed_reborrowable_parameters(cx, kind, body, local_def_id);
/// };
/// ```
pub fn borrowed_reborrowable_parameters<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    local_def_id: LocalDefId,
) -> impl Iterator<Item = (usize, Reborrowable)> {
    let signature = function_signature(cx, kind, local_def_id);
    let signature = signature.skip_binder();
    let output = signature.output();

    signature
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let ty::Ref(region, inner, Mutability::Mut) = input.kind() else {
                return None;
            };
            let is_self = body.params.get(index).is_some_and(|parameter| {
                matches!(
                    parameter.pat.kind,
                    PatKind::Binding(_, _, identifier, _) if identifier.name.as_str() == "self"
                )
            });
            if is_self || output.visit_with(&mut ContainsRegion(*region)).is_break() {
                return None;
            }

            reborrowable_type(cx, *inner).map(|reborrowable| (index, reborrowable))
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    function_signature(cx, kind, local_def_id)
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            query_data_type(cx, *input)
                .is_some_and(|query_data| direct_query_refs(query_data).any(|ty| is_zst(cx, ty)))
                .then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    let signature = function_signature(cx, kind, local_def_id);

    signature
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let query_data = query_data_type(cx, *input)?;
            if !query_data_has_any_mut_ref(query_data) {
                return None;
            }
            let parameter = body.params.get(index)?;
            let PatKind::Binding(_, binding_id, _, None) = parameter.pat.kind else {
                return None;
            };
            let mut visitor = QueryBindingUseVisitor {
                cx,
                binding_id,
                saw_read: false,
                saw_other_use: false,
            };
            visitor.visit_expr(body.value);
            (visitor.saw_read && !visitor.saw_other_use).then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    if !body_contains_fixed_entity_access(cx, body) {
        return Vec::new().into_iter();
    }

    function_signature(cx, kind, local_def_id)
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            query_data_type(cx, *input)
                .is_some_and(|data| {
                    type_is_named(cx, data, "bevy_ecs", "EntityRef")
                        || type_is_named(cx, data, "bevy_ecs", "EntityMut")
                })
                .then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
        .filter(|target| local_named_field_count(cx, *target) >= 8)
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
) -> impl Iterator<Item = usize> {
    let query_data_targets = local_trait_targets(cx, "bevy_ecs", "QueryData");

    function_signature(cx, kind, local_def_id)
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let data = query_data_type(cx, *input)?;
            let (total, mutable) = query_access_width(cx, data, &query_data_targets);
            let is_custom_limit_exceeded = local_adt_id(data)
                .is_some_and(|target| query_data_targets.contains(&target) && total > 8);
            (total > 5 || mutable > 4 || is_custom_limit_exceeded).then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    let query_data_targets = local_trait_targets(cx, "bevy_ecs", "QueryData");
    let mut visitor = FieldNameVisitor { names: Vec::new() };
    visitor.visit_expr(body.value);

    function_signature(cx, kind, local_def_id)
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let data = query_data_type(cx, *input)?;
            let target = local_adt_id(data)?;
            if !query_data_targets.contains(&target) {
                return None;
            }
            let fields = local_named_fields(cx, target);
            if fields.len() <= 8 {
                return None;
            }
            let used = fields
                .iter()
                .filter(|field| visitor.names.contains(field))
                .count();
            (used * 2 <= fields.len()).then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    let signature = function_signature(cx, kind, local_def_id);

    signature
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let data = query_data_type(cx, *input)?;
            if !matches!(data.kind(), ty::Ref(_, _, Mutability::Not)) {
                return None;
            }
            let parameter = body.params.get(index)?;
            let PatKind::Binding(_, binding_id, _, None) = parameter.pat.kind else {
                return None;
            };
            let mut visitor = PresenceQueryUseVisitor {
                cx,
                binding_id,
                saw_presence_use: false,
                saw_other_use: false,
            };
            visitor.visit_expr(body.value);
            (visitor.saw_presence_use && !visitor.saw_other_use).then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    if !matches!(kind, FnKind::ItemFn(..)) {
        return Vec::new().into_iter();
    }
    let signature = function_signature(cx, kind, local_def_id);

    signature
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let ty::Ref(_, inner, Mutability::Mut) = input.kind() else {
                return None;
            };
            if !type_is_named(cx, *inner, "bevy_ecs", "World") {
                return None;
            }
            let parameter = body.params.get(index)?;
            let PatKind::Binding(_, binding_id, _, None) = parameter.pat.kind else {
                return None;
            };
            let mut visitor = WorldBindingUseVisitor {
                cx,
                binding_id,
                saw_narrow_access: false,
                saw_other_use: false,
            };
            visitor.visit_expr(body.value);
            (visitor.saw_narrow_access && !visitor.saw_other_use).then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
) -> impl Iterator<Item = usize> {
    let large_components = local_large_components(cx).collect::<Vec<_>>();
    let mut visitor = FieldNameVisitor { names: Vec::new() };
    visitor.visit_expr(body.value);

    function_signature(cx, kind, local_def_id)
        .skip_binder()
        .inputs()
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let filter = query_filter_type(cx, *input)?;
            let component = query_filter_tracked_component(cx, filter, &large_components)?;
            let fields = local_named_fields(cx, component);
            let used = fields
                .iter()
                .filter(|field| visitor.names.contains(field))
                .count();
            (used * 2 < fields.len()).then_some(index)
        })
        .collect::<Vec<_>>()
        .into_iter()
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
    let components = function_signature(cx, kind, local_def_id)
        .skip_binder()
        .inputs()
        .iter()
        .filter_map(|input| query_data_type(cx, *input))
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

/// Return direct local systems from one resolved `App::add_systems` call.
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
    // Keep only direct local function paths because later analysis needs their bodies.
    let systems = direct_system_expressions(systems)
        .filter_map(|system| {
            let ExprKind::Path(ref path) = system.kind else {
                return None;
            };
            let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, system.hir_id) else {
                return None;
            };
            def_id.as_local()
        })
        .collect::<Vec<_>>();
    (!systems.is_empty()).then_some(RegisteredSystems { schedule, systems })
}

/// Return whether a direct system function mutably queries camera-filtered entities.
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

/// Return spans for direct systems that mutate a camera in `FixedUpdate`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::camera_fixed_update_system_spans(cx, expr);
/// };
/// ```
pub fn camera_fixed_update_system_spans(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> impl Iterator<Item = Span> {
    // Require the exact registration method and the fixed-update schedule.
    let Some(call) = app_method_call(cx, expr, "add_systems") else {
        return Vec::new().into_iter();
    };
    let [schedule, systems, ..] = call.arguments else {
        return Vec::new().into_iter();
    };
    if !expression_has_type(cx, schedule, "bevy_app", "FixedUpdate") {
        return Vec::new().into_iter();
    }

    // Report only direct systems whose signatures prove mutable camera access.
    direct_system_expressions(systems)
        .filter_map(|system| {
            let ExprKind::Path(ref path) = system.kind else {
                return None;
            };
            let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, system.hir_id) else {
                return None;
            };
            is_system_mutably_querying_camera(cx, def_id).then_some(system.span)
        })
        .collect::<Vec<_>>()
        .into_iter()
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

/// Return unit expression spans passed to known Bevy bundle methods.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::unit_bundle_spans(cx, expr);
/// };
/// ```
pub fn unit_bundle_spans(cx: &LateContext<'_>, expr: &Expr<'_>) -> Vec<Span> {
    // Resolve only method calls before inspecting receiver-specific bundle behavior.
    let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
        return Vec::new();
    };
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return Vec::new();
    };
    if cx.tcx.crate_name(def_id.krate).as_str() != "bevy_ecs" {
        return Vec::new();
    }

    // Restrict the accepted API set to methods whose first argument is a bundle.
    let receiver_name = adt_name(
        cx,
        cx.typeck_results().expr_ty_adjusted(receiver).peel_refs(),
    );
    let method_symbol = cx.tcx.item_name(def_id);
    let method_name = method_symbol.as_str();
    let is_bundle_accepted = receiver_name.is_some_and(|name| {
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
    // A missing bundle argument cannot contain a unit value.
    let Some(bundle) = arguments.first() else {
        return Vec::new();
    };

    // Preserve every nested unit span so diagnostics can target the smallest expression.
    let mut spans = Vec::new();
    collect_unit_expression_spans(cx.typeck_results().expr_ty(bundle), bundle, &mut spans);
    spans
}

/// Return the duplicate `add_plugins` method span for adjacent chained calls.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::duplicate_plugin_addition_span(cx, expr);
/// };
/// ```
pub fn duplicate_plugin_addition_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Require adjacent resolved registration calls before comparing their arguments.
    let outer = app_method_call(cx, expr, "add_plugins")?;
    let inner = app_method_call(cx, outer.receiver, "add_plugins")?;
    let outer_plugin = outer.arguments.first()?;
    let inner_plugin = inner.arguments.first()?;
    let outer_ty = cx.typeck_results().expr_ty(outer_plugin);
    let inner_ty = cx.typeck_results().expr_ty(inner_plugin);

    // Default plugin uniqueness makes equal adjacent plugin types duplicates.
    (outer_ty == inner_ty && local_plugin_uses_default_uniqueness(cx, outer_ty))
        .then_some(outer.method_span)
}

/// Return the method span when `Time::elapsed_secs()` is immediately cast to `f64`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = bevy_support::elapsed_secs_cast_f64_span(cx, expr);
/// };
/// ```
pub fn elapsed_secs_cast_f64_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Require an explicit cast to `f64` before resolving the source method.
    let ExprKind::Cast(operand, _) = expr.kind else {
        return None;
    };
    if !matches!(
        cx.typeck_results().expr_ty(expr).kind(),
        ty::Float(ty::FloatTy::F64)
    ) {
        return None;
    }

    bevy_method_call(cx, operand, "bevy_time", "Time", "elapsed_secs").map(|call| call.method_span)
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
    let implemented = local_standard_trait_targets(cx, marker_trait.name());

    local_trait_targets(cx, "bevy_ecs", "Component")
        .into_iter()
        .filter(|target| local_item_is_unit_struct(cx, *target))
        .filter(move |target| !implemented.contains(target))
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

/// Count loaded crates whose original metadata name is `bevy`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::loaded_bevy_facades(cx);
/// };
/// ```
pub fn loaded_bevy_facades(cx: &LateContext<'_>) -> usize {
    cx.tcx
        .crates(())
        .iter()
        .filter(|&&crate_num| cx.tcx.crate_name(crate_num).as_str() == "bevy")
        .count()
}

/// Return whether a unit-returning entrypoint contains a discarded `App::run`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, kind, declaration, body, _local_def_id| {
///     let _ = bevy_support::discarded_app_run_spans(cx, kind, declaration, body, _local_def_id);
/// };
/// ```
pub fn discarded_app_run_spans<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    declaration: &FnDecl<'tcx>,
    body: &Body<'tcx>,
    _local_def_id: LocalDefId,
) -> Vec<Span> {
    // Only named free-function items with a unit declaration can be this entrypoint.
    let FnKind::ItemFn(identifier, ..) = kind else {
        return Vec::new();
    };
    if identifier.name.as_str() != "main"
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

/// Declare a lint whose semantic matcher returns one expression span and a fixed replacement.
#[macro_export]
macro_rules! declare_expression_span_suggestion_lint {
    (
        $lint:ident, $pass:ident, $level:ident, $checker:path, $description:literal,
        $message:literal, $help:literal, $replacement:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            $level,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check one semantically resolved Bevy expression and offer its exact replacement.
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
                            diagnostic.primary_message($message).span_suggestion(
                                span,
                                $help,
                                $replacement,
                                rustc_errors::Applicability::MachineApplicable,
                            );
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

/// Declare a lint whose semantic matcher returns multiple expression spans.
#[macro_export]
macro_rules! declare_expression_spans_lint {
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
            /// Check semantically resolved Bevy expressions.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                for span in $checker(cx, expr) {
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
                body: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                for index in $crate::mutable_query_component_parameters(
                    cx,
                    kind,
                    body,
                    local_def_id,
                    $component_crate,
                    $component,
                ) {
                    let Some(parameter) = declaration.inputs.get(index) else {
                        continue;
                    };
                    cx.emit_span_lint(
                        $lint,
                        parameter.span,
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

                for index in $checker(cx, kind, body, local_def_id) {
                    let Some(parameter) = declaration.inputs.get(index) else {
                        continue;
                    };
                    cx.emit_span_lint(
                        $lint,
                        parameter.span,
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

                for index in $checker(cx, kind, local_def_id) {
                    let Some(parameter) = declaration.inputs.get(index) else {
                        continue;
                    };
                    cx.emit_span_lint(
                        $lint,
                        parameter.span,
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

                for target in $crate::local_unit_components_missing_trait(
                    cx,
                    $crate::MarkerTrait::$marker_trait,
                ) {
                    cx.emit_span_lint(
                        $lint,
                        cx.tcx.def_span(target),
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

/// Return a function signature for a free function, method, or closure.
fn function_signature<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    local_def_id: LocalDefId,
) -> ty::PolyFnSig<'tcx> {
    match kind {
        FnKind::Closure => cx.tcx.closure_user_provided_sig(local_def_id).value,
        FnKind::ItemFn(..) | FnKind::Method(..) => cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    }
}

/// Return query data from an instantiated `Query` type.
fn query_data_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(definition, arguments) = ty.peel_refs().kind() else {
        return None;
    };
    (cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
        && cx.tcx.item_name(definition.did()).as_str() == "Query")
        .then(|| arguments.types().next())
        .flatten()
}

/// Return query filters from an instantiated `Query` type.
fn query_filter_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(definition, arguments) = ty.peel_refs().kind() else {
        return None;
    };
    (cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
        && cx.tcx.item_name(definition.did()).as_str() == "Query")
        .then(|| arguments.types().nth(1))
        .flatten()
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

/// Return whether query data includes a mutable reference to one exact component.
fn query_data_has_mut_component<'tcx>(
    cx: &LateContext<'tcx>,
    query_ty: Ty<'tcx>,
    component_crate: &str,
    component_name: &str,
) -> bool {
    query_data_type(cx, query_ty).is_some_and(|data| {
        query_data_mut_components(data)
            .any(|component| type_is_named(cx, component, component_crate, component_name))
    })
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
    // Short-circuit on a mutable leaf before searching tuple elements.
    if let ty::Ref(_, _, Mutability::Mut) = ty.kind() {
        return true;
    }
    if let ty::Tuple(elements) = ty.kind() {
        return elements.iter().any(query_data_has_any_mut_ref);
    }
    false
}

/// Return whether a query filter includes `With<Camera>`.
fn query_filter_has_camera(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    // Search tuple filters recursively before inspecting one ADT filter.
    if let ty::Tuple(elements) = ty.kind() {
        return elements
            .iter()
            .any(|element| query_filter_has_camera(cx, element));
    }
    if let ty::Adt(definition, arguments) = ty.kind() {
        return cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
            && cx.tcx.item_name(definition.did()).as_str() == "With"
            && arguments
                .types()
                .any(|argument| type_is_named(cx, argument, "bevy_camera", "Camera"));
    }
    false
}

/// Return the local ADT definition represented by a type.
fn local_adt_id(ty: Ty<'_>) -> Option<LocalDefId> {
    let ty::Adt(definition, _) = ty.peel_refs().kind() else {
        return None;
    };
    definition.did().as_local()
}

/// Return the number of named fields on a local struct.
fn local_named_field_count(cx: &LateContext<'_>, local_def_id: LocalDefId) -> usize {
    local_named_fields(cx, local_def_id).len()
}

/// Return the names of fields on a local named-field struct.
fn local_named_fields(cx: &LateContext<'_>, local_def_id: LocalDefId) -> Vec<Symbol> {
    let item = cx.tcx.hir_expect_item(local_def_id);
    let ItemKind::Struct(_, _, VariantData::Struct { fields, .. }) = item.kind else {
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
    if let ty::Adt(definition, arguments) = ty.kind() {
        let is_query_data = definition
            .did()
            .as_local()
            .is_some_and(|target| query_data_targets.contains(&target));
        if is_query_data {
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
    if let ty::Adt(definition, arguments) = ty.kind()
        && cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
        && cx.tcx.item_name(definition.did()).as_str() == "Changed"
    {
        return arguments.types().find_map(|argument| {
            local_adt_id(argument).filter(|target| components.contains(target))
        });
    }
    None
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

/// Yield direct system expressions, flattening only a tuple literal.
fn direct_system_expressions<'hir>(
    expr: &'hir Expr<'hir>,
) -> impl Iterator<Item = &'hir Expr<'hir>> {
    let systems = if let ExprKind::Tup(elements) = expr.kind {
        elements.iter().collect()
    } else {
        vec![expr]
    };
    systems.into_iter()
}

/// Resolve an exact `App` or `SubApp` method.
fn app_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    method_name: &str,
) -> Option<BevyMethodCall<'hir>> {
    // Resolve the call before comparing its defining crate and receiver type.
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    let receiver_ty = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs();
    // Accept only the exact Bevy app APIs used by the lint suite.
    if cx.tcx.crate_name(def_id.krate).as_str() != "bevy_app"
        || cx.tcx.item_name(def_id).as_str() != method_name
        || !adt_name(cx, receiver_ty).is_some_and(|name| matches!(name.as_str(), "App" | "SubApp"))
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

/// Return an ADT's item name.
fn adt_name(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Symbol> {
    let ty::Adt(definition, _) = ty.kind() else {
        return None;
    };
    Some(cx.tcx.item_name(definition.did()))
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
    let crate_name = crate_symbol.as_str();
    let type_name = type_symbol.as_str();

    // Keep the allowlist closed so unrelated names cannot gain reborrow semantics.
    match (crate_name, type_name) {
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

/// Collect unit leaves and align them with tuple expressions when possible.
fn collect_unit_expression_spans(ty: Ty<'_>, expr: &Expr<'_>, spans: &mut Vec<Span>) {
    // Unit values can occur only in tuple-shaped bundle types.
    let ty::Tuple(elements) = ty.kind() else {
        return;
    };
    if elements.is_empty() {
        spans.push(expr.span);
        return;
    }

    // Recurse into aligned tuple literals to retain precise source spans.
    if let ExprKind::Tup(expressions) = expr.kind
        && expressions.len() == elements.len()
    {
        for (element, expression) in elements.iter().zip(expressions) {
            collect_unit_expression_spans(element, expression, spans);
        }
    } else if tuple_type_contains_unit(ty) {
        spans.push(expr.span);
    }
}

/// Return whether a nested tuple contains unit.
fn tuple_type_contains_unit(ty: Ty<'_>) -> bool {
    let ty::Tuple(elements) = ty.kind() else {
        return false;
    };
    elements.is_empty() || elements.iter().any(tuple_type_contains_unit)
}

/// Return local ADT targets implementing one exact trait.
fn local_trait_targets(
    cx: &LateContext<'_>,
    trait_crate: &str,
    trait_name: &str,
) -> Vec<LocalDefId> {
    // Walk resolved local implementations instead of relying on trait spelling.
    let mut targets = Vec::new();

    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        // Discard implementations of traits outside the target crate boundary.
        if !trait_is_named(cx, trait_def_id, trait_crate, trait_name) {
            continue;
        }
        // Retain each local ADT once even when duplicate impl metadata is present.
        for &impl_def_id in impl_def_ids {
            if !matches!(
                cx.tcx.def_kind(impl_def_id),
                rustc_hir::def::DefKind::Impl { .. }
            ) {
                continue;
            }
            let self_ty = cx
                .tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip();
            let ty::Adt(definition, _) = self_ty.kind() else {
                continue;
            };
            if let Some(local_def_id) = definition.did().as_local()
                && !targets.contains(&local_def_id)
            {
                targets.push(local_def_id);
            }
        }
    }

    targets
}

/// Return local ADT targets implementing one standard-library trait.
fn local_standard_trait_targets(cx: &LateContext<'_>, trait_name: &str) -> Vec<LocalDefId> {
    // Search only resolved traits from the standard library crates.
    let mut targets = Vec::new();

    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        // Name and crate checks jointly exclude unrelated traits with the same spelling.
        if cx.tcx.item_name(trait_def_id).as_str() != trait_name
            || !matches!(
                cx.tcx.crate_name(trait_def_id.krate).as_str(),
                "core" | "std"
            )
        {
            continue;
        }
        // Normalize implementation targets to unique local ADT identifiers.
        for &impl_def_id in impl_def_ids {
            if !matches!(
                cx.tcx.def_kind(impl_def_id),
                rustc_hir::def::DefKind::Impl { .. }
            ) {
                continue;
            }
            let self_ty = cx
                .tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip();
            let ty::Adt(definition, _) = self_ty.kind() else {
                continue;
            };
            if let Some(local_def_id) = definition.did().as_local()
                && !targets.contains(&local_def_id)
            {
                targets.push(local_def_id);
            }
        }
    }

    targets
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
    let Some(local_target) = definition.did().as_local() else {
        return false;
    };

    // Find the exact Bevy `Plugin` implementation for this local type.
    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        if !trait_is_named(cx, trait_def_id, "bevy_app", "Plugin") {
            continue;
        }
        // Ignore implementations for every other local plugin type.
        for &impl_def_id in impl_def_ids {
            if !matches!(
                cx.tcx.def_kind(impl_def_id),
                rustc_hir::def::DefKind::Impl { .. }
            ) {
                continue;
            }
            let impl_ty = cx
                .tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip();
            let ty::Adt(impl_definition, _) = impl_ty.kind() else {
                continue;
            };
            if impl_definition.did().as_local() != Some(local_target) {
                continue;
            }

            // Any explicit `is_unique` method replaces the trait's default behavior.
            return !cx
                .tcx
                .associated_items(impl_def_id)
                .in_definition_order()
                .any(|item| item.name().as_str() == "is_unique");
        }
    }

    false
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
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Classify resolved methods called directly on the tracked query binding.
        if let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind
            && local_path_id(self.cx, receiver) == Some(self.binding_id)
            && expression_has_type(self.cx, receiver, "bevy_ecs", "Query")
        {
            // The supported method set observes query data without mutable access.
            self.saw_read |= matches!(
                segment.ident.name.as_str(),
                "as_readonly"
                    | "contains"
                    | "get"
                    | "get_many"
                    | "is_empty"
                    | "iter"
                    | "iter_combinations"
                    | "iter_many"
                    | "many"
                    | "par_iter"
                    | "single"
            );
            self.saw_other_use |= !matches!(
                segment.ident.name.as_str(),
                "as_readonly"
                    | "contains"
                    | "get"
                    | "get_many"
                    | "is_empty"
                    | "iter"
                    | "iter_combinations"
                    | "iter_many"
                    | "many"
                    | "par_iter"
                    | "single"
            );
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
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Stop traversal after the first qualifying access has established the result.
        if self.is_found {
            return;
        }
        // Match only fixed typed access methods on the two supported entity proxies.
        if let ExprKind::MethodCall(segment, receiver, _, _) = expr.kind
            && (expression_has_type(self.cx, receiver, "bevy_ecs", "EntityRef")
                || expression_has_type(self.cx, receiver, "bevy_ecs", "EntityMut"))
            && matches!(
                segment.ident.name.as_str(),
                "contains"
                    | "get"
                    | "get_components"
                    | "get_components_mut"
                    | "get_mut"
                    | "get_ref"
            )
        {
            self.is_found = true;
            return;
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Visitor that records every named field access in a function body.
struct FieldNameVisitor {
    /// Unique field names used by field expressions.
    names: Vec<Symbol>,
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

impl<'tcx> Visitor<'tcx> for FieldNameVisitor {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Field(_, identifier) = expr.kind
            && !self.names.contains(&identifier.name)
        {
            self.names.push(identifier.name);
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
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Classify direct methods on the tracked query before visiting nested arguments.
        if let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind {
            if local_path_id(self.cx, receiver) == Some(self.binding_id)
                && expression_has_type(self.cx, receiver, "bevy_ecs", "Query")
            {
                // `is_empty` observes presence; every other direct method exceeds that scope.
                if segment.ident.name.as_str() == "is_empty" {
                    self.saw_presence_use = true;
                } else {
                    self.saw_other_use = true;
                }
                // Skip the receiver to prevent its path from becoming an opaque use.
                for argument in arguments {
                    self.visit_expr(argument);
                }
                return;
            }
            // Treat `query.iter().count()` as another presence-only operation.
            if segment.ident.name.as_str() == "count"
                && let ExprKind::MethodCall(iter_segment, query, iter_arguments, _) = receiver.kind
                && iter_segment.ident.name.as_str() == "iter"
                && local_path_id(self.cx, query) == Some(self.binding_id)
                && expression_has_type(self.cx, query, "bevy_ecs", "Query")
            {
                self.saw_presence_use = true;
                for argument in iter_arguments.iter().chain(arguments) {
                    self.visit_expr(argument);
                }
                return;
            }
        }
        if local_path_id(self.cx, expr) == Some(self.binding_id) {
            self.saw_other_use = true;
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
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Recognize query-state methods that receive the tracked world as an argument.
        if let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind {
            let is_query_state = expression_has_type(self.cx, receiver, "bevy_ecs", "QueryState");
            if is_query_state {
                // Separate the world argument from nested expressions that still need traversal.
                let has_world_argument = arguments
                    .iter()
                    .any(|argument| local_path_id(self.cx, argument) == Some(self.binding_id));
                if has_world_argument {
                    self.saw_narrow_access = true;
                    self.visit_expr(receiver);
                    let cx = self.cx;
                    let binding_id = self.binding_id;
                    arguments
                        .iter()
                        .filter(|argument| local_path_id(cx, argument) != Some(binding_id))
                        .for_each(|argument| self.visit_expr(argument));
                    return;
                }
            }
        }
        // Classify methods called directly on the tracked world binding.
        if let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind
            && local_path_id(self.cx, receiver) == Some(self.binding_id)
            && expression_has_type(self.cx, receiver, "bevy_ecs", "World")
        {
            // The closed method set exposes narrow resource or entity access.
            let is_narrow = matches!(
                segment.ident.name.as_str(),
                "entities"
                    | "get"
                    | "get_mut"
                    | "get_resource"
                    | "get_resource_mut"
                    | "query"
                    | "query_filtered"
                    | "resource"
                    | "resource_mut"
            );
            self.saw_narrow_access |= is_narrow;
            self.saw_other_use |= !is_narrow;
            // Visit call arguments without reclassifying the receiver path.
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
