#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Detect overlapping Bevy query access in supported system parameter types.
//!
//! The late pass resolves built-in query data and filter types, expands bounded
//! `Or` filters into disjunctive branches, and tracks ordinary tuples and nested
//! `ParamSet` member scopes. It compares those accesses with supported resource
//! parameters and reports only functions proven to be directly registered as
//! Bevy systems. Unknown custom query data, filters, and parameter wrappers are
//! skipped because their access semantics are not available from resolved types.
//! Default disabling filters are approximated from the Bevy ECS crate version;
//! world-specific filter replacement or registration can change runtime results.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use dylint_linting as _;
use rustc_lint::LintContext as _;
use rustc_middle::ty::{self, Ty};
use rustc_span::{
    Span,
    def_id::{CrateNum, DefId, LocalDefId},
};

#[cfg(test)]
use {
    bevy_018 as _, bevy_018_system_fixture as _, bevy_app as _, bevy_ecs as _, bevy_support as _,
};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub BEVY_CONFLICTING_QUERY_PARAMS,
    Warn,
    "Bevy query parameters have conflicting access",
    BevyConflictingQueryParams,
    BevyConflictingQueryParams::default()
}

/// Maximum number of filter alternatives expanded by the supported DNF parser.
const MAX_FILTER_ALTERNATIVES: usize = 64;

/// A component access performed by supported query data.
#[derive(Clone, Copy, Debug)]
struct ComponentAccess<'tcx> {
    /// Resolved, instantiated component type.
    component: Ty<'tcx>,
    /// Whether query data writes the component.
    is_mutable: bool,
}

/// One positive or negative component constraint in a filter alternative.
#[derive(Clone, Copy, Debug)]
struct FilterConstraint<'tcx> {
    /// Resolved component type constrained by this filter.
    component: Ty<'tcx>,
    /// Whether the entity is required to contain the component.
    is_present: bool,
}

/// One conjunction in a filter expressed as disjunctive normal form.
type FilterBranch<'tcx> = Vec<FilterConstraint<'tcx>>;

/// One member scope inside a `ParamSet` system parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParamSetScope {
    /// Identity of the containing `ParamSet` parameter.
    set_id: ParamSetId,
    /// Tuple member index; different indices are accessed sequentially.
    member_index: usize,
}

/// Identity for one `ParamSet` parameter within a system signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParamSetId(usize);

/// Supported built-in query filter constructors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BevyQueryFilterKind {
    /// Require one component to be present.
    With,
    /// Require one component to be absent.
    Without,
    /// Match any tuple member's filter.
    Or,
}

impl std::str::FromStr for BevyQueryFilterKind {
    type Err = ();

    /// Parse the closed vocabulary of resolved Bevy filter definitions.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "With" => Ok(Self::With),
            "Without" => Ok(Self::Without),
            "Or" => Ok(Self::Or),
            _ => Err(()),
        }
    }
}

/// Supported built-in Bevy resource parameter wrappers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BevyResourceParameterKind {
    /// Shared send-resource access.
    Res,
    /// Mutable send-resource access.
    ResMut,
    /// Shared non-send-resource access.
    NonSend,
    /// Mutable non-send-resource access.
    NonSendMut,
}

impl std::str::FromStr for BevyResourceParameterKind {
    type Err = ();

    /// Parse the closed vocabulary of resolved Bevy resource wrappers.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "Res" => Ok(Self::Res),
            "ResMut" => Ok(Self::ResMut),
            "NonSend" => Ok(Self::NonSend),
            "NonSendMut" => Ok(Self::NonSendMut),
            _ => Err(()),
        }
    }
}

/// Resolved query accesses and their supported filters.
#[derive(Clone, Debug)]
struct QueryParameter<'tcx> {
    /// Component reads and writes performed by query data.
    accesses: Vec<ComponentAccess<'tcx>>,
    /// Alternative filter conjunctions used by Bevy's access compatibility checks.
    filters: Vec<FilterBranch<'tcx>>,
    /// Bevy ECS crate that defines this query parameter.
    ecs_crate: CrateNum,
    /// Relative `SystemParam::init_access` order in the system signature.
    parameter_order: usize,
    /// `ParamSet` declarations containing this query.
    param_sets: Vec<ParamSetScope>,
    /// Source span of the query system parameter.
    span: Span,
}

/// One resolved resource or non-send resource parameter.
#[derive(Clone, Debug)]
struct ResourceParameter<'tcx> {
    /// Resolved, instantiated resource type.
    resource: Ty<'tcx>,
    /// Whether the parameter requests mutable access.
    is_mutable: bool,
    /// Whether this is a non-send resource with unfiltered component access in Bevy 0.19.
    is_non_send: bool,
    /// Bevy ECS crate that defines this parameter.
    ecs_crate: CrateNum,
    /// Relative `SystemParam::init_access` order in the system signature.
    parameter_order: usize,
    /// `ParamSet` declarations containing this resource.
    param_sets: Vec<ParamSetScope>,
    /// Source span of the resource system parameter.
    span: Span,
}

/// The conflict kind determines the Bevy error code and help text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConflictKind {
    /// Two query parameters have overlapping access, corresponding to B0001.
    Pair,
    /// One query data type requests conflicting access to one component.
    QueryData,
    /// A query overlaps a `Res` or `ResMut`, corresponding to B0002.
    Resource,
    /// A query overlaps a `NonSend` or `NonSendMut`, corresponding to B0002.
    NonSend,
}

/// A detected conflict waiting for direct system registration evidence.
#[derive(Clone, Copy, Debug)]
struct PendingConflict {
    /// Function whose registration must be proven.
    system: LocalDefId,
    /// Location of the later conflicting parameter.
    span: Span,
    /// Bevy access conflict represented by this diagnostic.
    kind: ConflictKind,
}

/// Stateful late pass that records supported conflicts and emits them only after
/// resolved app-registration calls prove the functions are used as Bevy systems.
#[derive(Debug, Default)]
pub struct BevyConflictingQueryParams {
    /// Conflicts found in supported query and resource parameter shapes.
    conflicts: Vec<PendingConflict>,
    /// Free functions found in resolved `App::add_systems` calls.
    registered_systems: Vec<LocalDefId>,
}

/// State shared while collecting supported parameters from one system signature.
struct ParameterCollection<'a, 'tcx> {
    /// Rustc context used to resolve Bevy parameter types.
    cx: &'a rustc_lint::LateContext<'tcx>,
    /// Collected query accesses.
    queries: Vec<QueryParameter<'tcx>>,
    /// Collected resource accesses.
    resources: Vec<ResourceParameter<'tcx>>,
    /// Next `ParamSet` identity for this function.
    next_param_set: usize,
    /// Next parameter order for non-send access analysis.
    next_parameter_order: usize,
}

impl<'a, 'tcx> ParameterCollection<'a, 'tcx> {
    /// Create empty collection state for one system signature.
    const fn new(cx: &'a rustc_lint::LateContext<'tcx>) -> Self {
        Self {
            cx,
            queries: Vec::new(),
            resources: Vec::new(),
            next_param_set: 0,
            next_parameter_order: 0,
        }
    }

    /// Collect one parameter recursively through supported tuple and `ParamSet` forms.
    fn collect_supported(
        &mut self,
        parameter: Ty<'tcx>,
        span: Span,
        param_sets: &mut Vec<ParamSetScope>,
    ) {
        if is_query_parameter(
            self.cx,
            parameter,
            span,
            param_sets,
            &mut self.next_parameter_order,
            &mut self.queries,
        ) {
            return;
        }

        // Resource wrappers are recognized before generic tuple and ParamSet descent.
        if is_resource_parameter(
            self.cx,
            parameter,
            span,
            param_sets,
            &mut self.next_parameter_order,
            &mut self.resources,
        ) {
            return;
        }

        // Ordinary tuples request every contained system parameter at the same time.
        if let ty::TyKind::Tuple(elements) = parameter.kind() {
            for element in *elements {
                self.collect_supported(element, span, param_sets);
            }
            return;
        }

        self.collect_param_set_members(parameter, span, param_sets);
    }

    /// Collect parameters from tuple-valued `ParamSet` members as exclusive alternatives.
    fn collect_param_set_members(
        &mut self,
        parameter: Ty<'tcx>,
        span: Span,
        param_sets: &mut Vec<ParamSetScope>,
    ) {
        // ParamSet members are alternatives whose inner tuple accesses remain simultaneous.
        let Some((_, arguments)) = bevy_type_arguments(self.cx, parameter, "ParamSet") else {
            return;
        };
        let Some(parameters) = arguments.first() else {
            return;
        };
        let ty::TyKind::Tuple(parameters) = parameters.kind() else {
            return;
        };
        let param_set = ParamSetId(self.next_param_set);
        self.next_param_set += 1;
        // Track one scope per member and restore it after collecting that tuple.
        for (member_index, parameter) in parameters.iter().enumerate() {
            param_sets.push(ParamSetScope {
                set_id: param_set,
                member_index,
            });
            self.collect_supported(parameter, span, param_sets);
            // Keep only enclosing scopes after this alternative finishes.
            let _popped_scope = param_sets.pop();
        }
    }
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyConflictingQueryParams {
    /// Collect supported accesses from the resolved function signature.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        _: &'tcx rustc_hir::Body<'tcx>,
        _: Span,
        system: LocalDefId,
    ) {
        if matches!(kind, rustc_hir::intravisit::FnKind::Closure) {
            return;
        }

        let signature = cx
            .tcx
            .fn_sig(system.to_def_id())
            .instantiate_identity()
            .skip_binder();
        let mut collection = ParameterCollection::new(cx);

        // Walk direct parameters and ordinary tuples while preserving ParamSet scopes.
        for (parameter, parameter_type) in declaration.inputs.iter().zip(signature.inputs()) {
            collection.collect_supported(*parameter_type, parameter.span, &mut Vec::new());
        }

        // Record failures while the resolved signature and ParamSet scopes are available.
        check_query_internal_conflicts(&mut self.conflicts, system, &collection.queries);
        check_query_pair_conflicts(&mut self.conflicts, system, &collection.queries);
        check_query_resource_conflicts(
            cx,
            &mut self.conflicts,
            system,
            &collection.queries,
            &collection.resources,
        );
    }

    /// Record direct free-function registrations through the resolved Bevy app API.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expression: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        if let Some(registration) = bevy_support::directly_registered_systems(cx, expression) {
            self.registered_systems.extend(registration.systems);
        }
    }

    /// Emit diagnostics only for functions present in a direct registration.
    fn check_crate_post(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        // Registration evidence suppresses diagnostics for ordinary helper functions.
        for conflict in &self.conflicts {
            if !self.registered_systems.contains(&conflict.system) {
                continue;
            }
            let (message, help, note) = match conflict.kind {
                ConflictKind::Pair => (
                    "this Bevy system may have conflicting query access",
                    "add disjoint filters or put both queries in different members of the same `ParamSet`; accesses within one member remain simultaneous",
                    "Bevy may report this as B0001 when it initializes the system; world default query filters can change the result",
                ),
                ConflictKind::QueryData => (
                    "this Bevy query has conflicting component access",
                    "move these accesses into separate query parameters, then add disjoint filters or place them in different members of one `ParamSet`",
                    "Bevy panics while it initializes query data that requests shared and mutable access to the same component",
                ),
                ConflictKind::Resource => (
                    "this Bevy system may have conflicting query and resource access",
                    "add `Without<IsResource>` or put both parameters in different members of the same `ParamSet`; accesses within one member remain simultaneous",
                    "Bevy may report this as B0002 when it initializes the system; world default query filters can change the result",
                ),
                ConflictKind::NonSend => (
                    "this Bevy system may have conflicting query and non-send resource access",
                    "put both parameters in different members of the same `ParamSet`; accesses within one member remain simultaneous",
                    "Bevy may report this as B0002 when it initializes the system; world default query filters can change the result",
                ),
            };
            cx.emit_span_lint(
                BEVY_CONFLICTING_QUERY_PARAMS,
                conflict.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic =
                        diagnostic.primary_message(message).help(help).note(note);
                }),
            );
        }
    }
}

/// Record duplicate component access within one query's data.
fn check_query_internal_conflicts(
    conflicts: &mut Vec<PendingConflict>,
    system: LocalDefId,
    queries: &[QueryParameter<'_>],
) {
    for query in queries {
        // Compare each access only with later entries to avoid duplicate diagnostics.
        for (index, left) in query.accesses.iter().enumerate() {
            query
                .accesses
                .iter()
                .skip(index + 1)
                .filter(|right| {
                    left.component == right.component && (left.is_mutable || right.is_mutable)
                })
                .for_each(|_| {
                    push_conflict(conflicts, system, query.span, ConflictKind::QueryData);
                });
        }
    }
}

/// Record query-pair conflicts after applying `ParamSet` scopes and filter constraints.
fn check_query_pair_conflicts(
    conflicts: &mut Vec<PendingConflict>,
    system: LocalDefId,
    queries: &[QueryParameter<'_>],
) {
    // Query pairs are checked once, in source order, after ParamSet and filter exclusions.
    for (index, left) in queries.iter().enumerate() {
        for right in queries.iter().skip(index + 1) {
            // Different ParamSet members are sequential access alternatives.
            let is_deferred_to_separate_members =
                accesses_are_in_separate_param_set_members(&left.param_sets, &right.param_sets);
            // Opposite constraints on one component prove two filter branches disjoint.
            let can_filters_overlap = left.filters.iter().any(|left_branch| {
                right.filters.iter().any(|right_branch| {
                    !left_branch.iter().any(|left_constraint| {
                        right_branch.iter().any(|right_constraint| {
                            left_constraint.component == right_constraint.component
                                && left_constraint.is_present != right_constraint.is_present
                        })
                    })
                })
            });
            if is_deferred_to_separate_members || !can_filters_overlap {
                continue;
            }
            let has_conflicting_access = left.accesses.iter().any(|left_access| {
                right.accesses.iter().any(|right_access| {
                    left_access.component == right_access.component
                        && (left_access.is_mutable || right_access.is_mutable)
                })
            });
            if has_conflicting_access {
                push_conflict(conflicts, system, right.span, ConflictKind::Pair);
            }
        }
    }
}

/// Record query conflicts with resource parameters for one Bevy ECS version.
fn check_query_resource_conflicts<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    conflicts: &mut Vec<PendingConflict>,
    system: LocalDefId,
    queries: &[QueryParameter<'tcx>],
    resources: &[ResourceParameter<'tcx>],
) {
    // Each pair uses the resource's ECS crate to resolve its marker semantics.
    for query in queries {
        for resource in resources {
            check_query_resource_access(cx, conflicts, system, query, resource);
        }
    }
}

/// Check one query/resource pair using Bevy's version-specific resource marker.
fn check_query_resource_access<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    conflicts: &mut Vec<PendingConflict>,
    system: LocalDefId,
    query: &QueryParameter<'tcx>,
    resource: &ResourceParameter<'tcx>,
) {
    if query.ecs_crate != resource.ecs_crate
        || accesses_are_in_separate_param_set_members(&query.param_sets, &resource.param_sets)
    {
        return;
    }
    let Some(is_resource) = bevy_is_resource_marker(cx, resource.ecs_crate) else {
        return;
    };

    // Only same-type component and resource accesses can conflict in this pair.
    let has_conflicting_access = query.accesses.iter().any(|access| {
        access.component.eq(&resource.resource) && (access.is_mutable || resource.is_mutable)
    });
    if !has_conflicting_access {
        return;
    }

    // Filters may prove disjoint access for send resources, but not for ordered non-send access.
    let can_filters_overlap = query_resource_filters_can_overlap(query, resource, is_resource);
    if can_filters_overlap {
        // Report at the later declaration so the diagnostic points to the second access.
        push_conflict(
            conflicts,
            system,
            if resource.span.lo() >= query.span.lo() {
                resource.span
            } else {
                query.span
            },
            if resource.is_non_send {
                ConflictKind::NonSend
            } else {
                ConflictKind::Resource
            },
        );
    }
}

/// Return whether a query branch can overlap one component-backed resource access.
fn query_resource_filters_can_overlap<'tcx>(
    query: &QueryParameter<'tcx>,
    resource: &ResourceParameter<'tcx>,
    is_resource: Ty<'tcx>,
) -> bool {
    // Non-send access checks earlier parameters without considering query filters.
    if resource.is_non_send && resource.parameter_order >= query.parameter_order {
        return true;
    }

    query.filters.iter().any(|branch| {
        if resource.is_non_send {
            !is_component_excluded_by_filter_branch(branch, resource.resource)
        } else {
            !is_component_excluded_by_filter_branch(branch, is_resource)
                && !is_component_excluded_by_filter_branch(branch, resource.resource)
        }
    })
}

/// Return whether a branch excludes a component without also requiring it.
fn is_component_excluded_by_filter_branch<'tcx>(
    branch: &FilterBranch<'tcx>,
    component: Ty<'tcx>,
) -> bool {
    let has_positive_constraint = branch
        .iter()
        .any(|constraint| constraint.component == component && constraint.is_present);
    let has_negative_constraint = branch
        .iter()
        .any(|constraint| constraint.component == component && !constraint.is_present);
    has_negative_constraint && !has_positive_constraint
}

/// Collect one query parameter when its data and filter types are supported.
fn is_query_parameter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    parameter: Ty<'tcx>,
    span: Span,
    param_sets: &[ParamSetScope],
    next_parameter_order: &mut usize,
    queries: &mut Vec<QueryParameter<'tcx>>,
) -> bool {
    // A resolved Query wrapper is recognized even when its custom contents are unsupported.
    let Some((definition, arguments)) = bevy_type_arguments(cx, parameter, "Query") else {
        return false;
    };
    // Query arguments determine access data first and filter compatibility second.
    let parameter_order = *next_parameter_order;
    *next_parameter_order += 1;
    let mut arguments = arguments.into_iter();
    let Some(data_type) = arguments.next() else {
        return true;
    };
    let filter_type = arguments.next().unwrap_or_else(|| cx.tcx.types.unit);
    let Some(accesses) = parse_query_data(cx, data_type) else {
        return true;
    };
    let Some(mut filters) = parse_query_filter(cx, filter_type) else {
        return true;
    };
    let ecs_crate = definition.krate;
    let disabled = bevy_disabled_marker(cx, ecs_crate);

    // Bevy 0.18 and 0.19 standard worlds exclude Disabled unless the query mentions it.
    if let Some(disabled) = disabled {
        let is_disabled_mentioned = accesses.iter().any(|access| access.component == disabled)
            || filters
                .iter()
                .flatten()
                .any(|constraint| constraint.component == disabled);
        if !is_disabled_mentioned {
            // Default world access excludes disabled entities unless the query mentions that marker.
            let constraint = FilterConstraint {
                component: disabled,
                is_present: false,
            };
            for branch in &mut filters {
                branch.extend([constraint]);
            }
        }
    }
    // Record only parameter shapes with supported query data and filter semantics.
    queries.push(QueryParameter {
        accesses,
        filters,
        ecs_crate,
        parameter_order,
        param_sets: param_sets.to_vec(),
        span,
    });
    true
}

/// Collect one supported send or non-send resource parameter.
fn is_resource_parameter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    parameter: Ty<'tcx>,
    span: Span,
    param_sets: &[ParamSetScope],
    next_parameter_order: &mut usize,
    resources: &mut Vec<ResourceParameter<'tcx>>,
) -> bool {
    // A resolved resource wrapper is recognized before tuple and ParamSet traversal.
    let Some((resource, is_mutable, is_non_send, ecs_crate)) = resource_parameter(cx, parameter)
    else {
        return false;
    };
    let parameter_order = *next_parameter_order;
    *next_parameter_order += 1;
    // Preserve system-signature order for Bevy's NonSend access checks.
    resources.push(ResourceParameter {
        resource,
        is_mutable,
        is_non_send,
        ecs_crate,
        parameter_order,
        param_sets: param_sets.to_vec(),
        span,
    });
    true
}

/// Return component accesses for references, tuples, and Bevy's `Entity` query data.
fn parse_query_data<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    data: Ty<'tcx>,
) -> Option<Vec<ComponentAccess<'tcx>>> {
    // References declare the component access and mutability directly.
    if let ty::TyKind::Ref(_, component, mutability) = data.kind() {
        return Some(vec![ComponentAccess {
            component: *component,
            is_mutable: *mutability == rustc_hir::Mutability::Mut,
        }]);
    }

    if let ty::TyKind::Tuple(elements) = data.kind() {
        let mut accesses = Vec::new();
        // Every tuple member participates in the same query access set.
        for element in *elements {
            accesses.extend(parse_query_data(cx, element)?);
        }
        return Some(accesses);
    }

    // Entity access reads no component and is the only supported non-reference leaf.
    if let ty::TyKind::Adt(definition, _) = data.kind()
        && cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
        && cx.tcx.item_name(definition.did()).as_str() == "Entity"
    {
        return Some(Vec::new());
    }

    None
}

/// Parse supported conjunctions and disjunctions into bounded DNF branches.
fn parse_query_filter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    filter: Ty<'tcx>,
) -> Option<Vec<FilterBranch<'tcx>>> {
    if let ty::TyKind::Tuple(elements) = filter.kind() {
        return parse_tuple_query_filter(cx, elements);
    }

    parse_adt_query_filter(cx, filter)
}

/// Combine tuple members as a conjunction of filter alternatives.
fn parse_tuple_query_filter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    elements: &'tcx ty::List<Ty<'tcx>>,
) -> Option<Vec<FilterBranch<'tcx>>> {
    // Begin with the unconstrained conjunction for an empty filter tuple.
    let mut alternatives = vec![Vec::new()];
    // Each tuple member narrows all branches accumulated from earlier members.
    for element in elements {
        let element_alternatives = parse_query_filter(cx, element)?;
        alternatives = combine_filter_alternatives(&alternatives, &element_alternatives)?;
    }
    Some(alternatives)
}

/// Parse one resolved `With`, `Without`, or `Or` filter constructor.
fn parse_adt_query_filter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    filter: Ty<'tcx>,
) -> Option<Vec<FilterBranch<'tcx>>> {
    // Resolve only built-in Bevy constructors before interpreting their generic payload.
    let (kind, arguments) = resolved_query_filter_arguments(cx, filter)?;
    let component_or_alternatives = arguments.first().copied()?;
    match kind {
        BevyQueryFilterKind::With | BevyQueryFilterKind::Without => {
            parse_component_query_filter(component_or_alternatives, kind)
        }
        BevyQueryFilterKind::Or => parse_or_query_filter(cx, component_or_alternatives),
    }
}

/// Resolve one built-in Bevy filter definition and its type arguments.
fn resolved_query_filter_arguments<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    filter: Ty<'tcx>,
) -> Option<(BevyQueryFilterKind, Vec<Ty<'tcx>>)> {
    // Custom ADTs and non-ADT filters have unknown matching behavior.
    let ty::TyKind::Adt(definition, arguments) = filter.kind() else {
        return None;
    };
    if cx.tcx.crate_name(definition.did().krate).as_str() != "bevy_ecs" {
        return None;
    }
    // Parse only the closed constructor vocabulary resolved from this Bevy crate.
    let kind = cx
        .tcx
        .item_name(definition.did())
        .as_str()
        .parse::<BevyQueryFilterKind>()
        .ok()?;
    Some((kind, arguments.types().collect()))
}

/// Convert one `With<T>` or `Without<T>` type into a filter branch.
fn parse_component_query_filter(
    component: Ty<'_>,
    kind: BevyQueryFilterKind,
) -> Option<Vec<FilterBranch<'_>>> {
    let (BevyQueryFilterKind::With | BevyQueryFilterKind::Without) = kind else {
        return None;
    };
    Some(vec![vec![FilterConstraint {
        component,
        is_present: kind == BevyQueryFilterKind::With,
    }]])
}

/// Expand tuple alternatives in `Or` while respecting the analysis bound.
fn parse_or_query_filter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    filters: Ty<'tcx>,
) -> Option<Vec<FilterBranch<'tcx>>> {
    // Or accepts only the tuple payload used by Bevy's built-in filter type.
    let ty::TyKind::Tuple(filters) = filters.kind() else {
        return None;
    };
    let mut alternatives = Vec::new();
    // Preserve branch order while refusing expansions above the static bound.
    // Keep the expansion bounded before extending the accumulated branch list.
    for alternative in *filters {
        let branches = parse_query_filter(cx, alternative)?;
        if alternatives.len().saturating_add(branches.len()) > MAX_FILTER_ALTERNATIVES {
            return None;
        }
        alternatives.extend(branches);
    }
    Some(alternatives)
}

/// Form a bounded conjunction product for two DNF filter lists.
fn combine_filter_alternatives<'tcx>(
    left: &[FilterBranch<'tcx>],
    right: &[FilterBranch<'tcx>],
) -> Option<Vec<FilterBranch<'tcx>>> {
    // Reject large products before allocating their expanded branches.
    if left.len().saturating_mul(right.len()) > MAX_FILTER_ALTERNATIVES {
        return None;
    }
    // The bound above caps the collected DNF product at a small fixed size.
    Some(
        left.iter()
            .flat_map(|left_branch| {
                right.iter().map(move |right_branch| {
                    left_branch.iter().chain(right_branch).copied().collect()
                })
            })
            .collect(),
    )
}

/// Resolve a direct Bevy resource or non-send resource access.
fn resource_parameter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    parameter: Ty<'tcx>,
) -> Option<(Ty<'tcx>, bool, bool, CrateNum)> {
    // Only resolved Bevy ECS ADTs have the supported resource access contract.
    let ty::TyKind::Adt(definition, arguments) = parameter.kind() else {
        return None;
    };
    if cx.tcx.crate_name(definition.did().krate).as_str() != "bevy_ecs" {
        return None;
    }
    // Parse the closed wrapper set at the semantic type-resolution boundary.
    let kind = cx
        .tcx
        .item_name(definition.did())
        .as_str()
        .parse::<BevyResourceParameterKind>()
        .ok()?;
    let is_mutable = matches!(
        kind,
        BevyResourceParameterKind::ResMut | BevyResourceParameterKind::NonSendMut
    );
    let is_non_send = matches!(
        kind,
        BevyResourceParameterKind::NonSend | BevyResourceParameterKind::NonSendMut
    );
    // Missing resource type arguments are unsupported rather than guessed.
    Some((
        arguments.types().next()?,
        is_mutable,
        is_non_send,
        definition.did().krate,
    ))
}

/// Return type arguments for one resolved Bevy ECS ADT.
fn bevy_type_arguments<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    ty: Ty<'tcx>,
    expected_name: &str,
) -> Option<(DefId, Vec<Ty<'tcx>>)> {
    let ty::TyKind::Adt(definition, arguments) = ty.kind() else {
        return None;
    };
    (cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
        && cx.tcx.item_name(definition.did()).as_str() == expected_name)
        .then(|| (definition.did(), arguments.types().collect()))
}

/// Resolve Bevy's component marker for resource entities from one ECS crate.
fn bevy_is_resource_marker<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    ecs_crate: CrateNum,
) -> Option<Ty<'tcx>> {
    // Resolve the exact crate's marker, since several Bevy versions can be loaded.
    let crate_root = ecs_crate.as_def_id();
    let resource_module = module_child(cx, crate_root, "resource")?;
    let is_resource = module_child(cx, resource_module, "IsResource")?;
    let _resource_component_id = module_child(cx, resource_module, "IS_RESOURCE")?;
    Some(
        cx.tcx
            .type_of(is_resource)
            .instantiate_identity()
            .skip_norm_wip(),
    )
}

/// Resolve the default-disabled marker from one exact ECS crate.
///
/// This lookup does not depend on whether that version has resource markers.
fn bevy_disabled_marker<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    ecs_crate: CrateNum,
) -> Option<Ty<'tcx>> {
    let crate_root = ecs_crate.as_def_id();
    let entity_disabling = module_child(cx, crate_root, "entity_disabling")?;
    let disabled = module_child(cx, entity_disabling, "Disabled")?;
    Some(
        cx.tcx
            .type_of(disabled)
            .instantiate_identity()
            .skip_norm_wip(),
    )
}

/// Resolve a named direct child in the semantic module tree of one crate.
fn module_child(cx: &rustc_lint::LateContext<'_>, module: DefId, name: &str) -> Option<DefId> {
    cx.tcx
        .module_children(module)
        .iter()
        .find(|child| child.ident.name.as_str() == name)
        .and_then(|child| child.res.opt_def_id())
}

/// Return whether two accesses are in different members of one exclusive `ParamSet`.
fn accesses_are_in_separate_param_set_members(
    left: &[ParamSetScope],
    right: &[ParamSetScope],
) -> bool {
    left.iter().any(|left_scope| {
        right.iter().any(|right_scope| {
            left_scope.set_id == right_scope.set_id
                && left_scope.member_index != right_scope.member_index
        })
    })
}

/// Record a conflict once for one function, source span, and conflict category.
fn push_conflict(
    conflicts: &mut Vec<PendingConflict>,
    system: LocalDefId,
    span: Span,
    kind: ConflictKind,
) {
    if !conflicts
        .iter()
        .any(|conflict| conflict.system == system && conflict.span == span && conflict.kind == kind)
    {
        conflicts.push(PendingConflict { system, span, kind });
    }
}

#[cfg(test)]
/// Runtime checks for Bevy's query access initialization contract.
mod runtime_tests;

/// Run the UI cases for supported and skipped parameter shapes.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
