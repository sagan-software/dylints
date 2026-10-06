#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Detect overlapping access to resolved Bevy resources and non-send resources.
//!
//! The late pass resolves `Res`, `ResMut`, `NonSend`, and `NonSendMut` from each
//! function signature, then tracks accesses through ordinary tuples and nested
//! `ParamSet` members. It reports only conflicts that Bevy would reject during
//! system initialization and only after a direct app registration proves the
//! function is used as a system. Unsupported custom parameter wrappers are
//! skipped because their access behavior cannot be inferred from their names.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use dylint_linting as _;
use rustc_lint::LintContext as _;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub BEVY_CONFLICTING_RESOURCE_PARAMS,
    Warn,
    "Bevy resource parameters have conflicting access",
    BevyConflictingResourceParams,
    BevyConflictingResourceParams::default()
}

/// One known resource access declared by a supported system parameter.
#[derive(Clone, Debug)]
struct ResourceAccess<'tcx> {
    /// Resolved instantiated resource type.
    resource: Ty<'tcx>,
    /// Whether the parameter requests mutable access.
    is_mutable: bool,
    /// Nested `ParamSet` member scopes containing this access.
    param_sets: Vec<ParamSetScope>,
    /// Source span of this resource parameter type.
    span: Span,
}

/// The member of a `ParamSet` that contains one access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParamSetScope {
    /// Identity of one `ParamSet` parameter.
    set_id: ParamSetId,
    /// Tuple member index; only different members are mutually exclusive.
    member_index: usize,
}

/// Identity for one `ParamSet` parameter within a system signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParamSetId(usize);

/// A resolved resource parameter and whether it requests mutable access.
#[derive(Clone, Copy, Debug)]
enum ResourceParameter<'tcx> {
    /// Shared access through `Res` or `NonSend`.
    Shared(Ty<'tcx>),
    /// Mutable access through `ResMut` or `NonSendMut`.
    Mutable(Ty<'tcx>),
}

/// One diagnostic found while checking a function body.
#[derive(Clone, Copy, Debug)]
struct PendingConflict {
    /// Function whose direct system registration must be proven.
    system: LocalDefId,
    /// Parameter location for the diagnostic.
    span: Span,
}

/// Stateful pass that records supported resource pairs and emits diagnostics only
/// after a resolved Bevy app registration proves the function is an active system.
#[derive(Debug, Default)]
pub struct BevyConflictingResourceParams {
    /// Conflicts recorded from supported parameter shapes.
    conflicts: Vec<PendingConflict>,
    /// Free functions found in resolved `App::add_systems` calls.
    registered_systems: Vec<LocalDefId>,
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyConflictingResourceParams {
    /// Collect supported resource access and defer diagnostics until registration is known.
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

        let mut accesses = Vec::new();
        let mut next_param_set = 0;
        let signature = cx
            .tcx
            .fn_sig(system.to_def_id())
            .instantiate_identity()
            .skip_binder();

        // Walk ordinary tuple parameters while keeping each ParamSet as an access boundary.
        for (parameter, parameter_type) in declaration.inputs.iter().zip(signature.inputs()) {
            collect_resource_accesses(
                cx,
                *parameter_type,
                parameter.span,
                &mut Vec::new(),
                &mut next_param_set,
                &mut accesses,
            );
        }

        // Pair checks run after collection has retained nested ParamSet scopes.
        check_resource_conflicts(&mut self.conflicts, system, &accesses);
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
        for conflict in &self.conflicts {
            if !self.registered_systems.contains(&conflict.system) {
                continue;
            }
            cx.emit_span_lint(
                BEVY_CONFLICTING_RESOURCE_PARAMS,
                conflict.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(
                            "this Bevy system has conflicting access to one resource",
                        )
                        .help(
                            "combine the accesses or put both parameters in different members of the same `ParamSet`; accesses within one member remain simultaneous",
                        )
                        .note("Bevy reports this access conflict as B0002 when it initializes the system");
                }),
            );
        }
    }
}

/// Collect known resource access from direct parameters, tuples, and tuple `ParamSet`s.
fn collect_resource_accesses<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    parameter: Ty<'tcx>,
    span: Span,
    param_sets: &mut Vec<ParamSetScope>,
    next_param_set: &mut usize,
    accesses: &mut Vec<ResourceAccess<'tcx>>,
) {
    // Resource access wrappers are identified by their resolved Bevy definition.
    if let Some(resource_parameter) = resource_parameter(cx, parameter) {
        let resource = match resource_parameter {
            ResourceParameter::Shared(resource) | ResourceParameter::Mutable(resource) => resource,
        };
        accesses.push(ResourceAccess {
            resource,
            is_mutable: matches!(resource_parameter, ResourceParameter::Mutable(_)),
            param_sets: param_sets.clone(),
            span,
        });
        return;
    }

    // Ordinary tuples request every contained system parameter together.
    if let ty::TyKind::Tuple(elements) = parameter.kind() {
        for element in *elements {
            collect_resource_accesses(cx, element, span, param_sets, next_param_set, accesses);
        }
        return;
    }

    // Different tuple members are alternatives; access within one member remains simultaneous.
    collect_param_set_resource_accesses(cx, parameter, span, param_sets, next_param_set, accesses);
}

/// Collect tuple parameters from `ParamSet` alternatives with their member identity.
fn collect_param_set_resource_accesses<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    parameter: Ty<'tcx>,
    span: Span,
    param_sets: &mut Vec<ParamSetScope>,
    next_param_set: &mut usize,
    accesses: &mut Vec<ResourceAccess<'tcx>>,
) {
    // Unknown ParamSet shapes do not reveal simultaneous member access safely.
    let Some(arguments) = bevy_type_arguments(cx, parameter, "ParamSet") else {
        return;
    };
    let Some(parameters) = arguments.first() else {
        return;
    };
    // Only tuple-shaped alternatives expose their member boundaries.
    let ty::TyKind::Tuple(parameters) = parameters.kind() else {
        return;
    };
    // Assign one identity to this ParamSet so only cross-member pairs are exempt.
    let param_set = ParamSetId(*next_param_set);
    *next_param_set += 1;
    for (member_index, parameter) in parameters.iter().enumerate() {
        param_sets.push(ParamSetScope {
            set_id: param_set,
            member_index,
        });
        // Tuple contents remain simultaneous within one nested member scope.
        collect_resource_accesses(cx, parameter, span, param_sets, next_param_set, accesses);
        // Restore parent scopes before collecting another alternative.
        let _popped_scope = param_sets.pop();
    }
}

/// Resolve shared or mutable access from a Bevy resource wrapper.
fn resource_parameter<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    parameter: Ty<'tcx>,
) -> Option<ResourceParameter<'tcx>> {
    // Only the Bevy ECS crate's resolved resource wrappers have known access semantics.
    let ty::TyKind::Adt(definition, arguments) = parameter.kind() else {
        return None;
    };
    // The first type argument identifies the resource accessed by Bevy.
    let resource = arguments.types().next()?;
    if cx.tcx.crate_name(definition.did().krate).as_str() != "bevy_ecs" {
        return None;
    }
    match cx.tcx.item_name(definition.did()).as_str() {
        "Res" | "NonSend" => Some(ResourceParameter::Shared(resource)),
        "ResMut" | "NonSendMut" => Some(ResourceParameter::Mutable(resource)),
        _ => None,
    }
}

/// Return type arguments when a type resolves to a named Bevy ECS definition.
fn bevy_type_arguments<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    ty: Ty<'tcx>,
    expected_name: &str,
) -> Option<Vec<Ty<'tcx>>> {
    let ty::TyKind::Adt(definition, arguments) = ty.kind() else {
        return None;
    };
    (cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_ecs"
        && cx.tcx.item_name(definition.did()).as_str() == expected_name)
        .then(|| arguments.types().collect())
}

/// Record each conflicting resource pair once at the later parameter location.
fn check_resource_conflicts(
    conflicts: &mut Vec<PendingConflict>,
    system: LocalDefId,
    accesses: &[ResourceAccess<'_>],
) {
    // Visit pairs in declaration order and report each conflict at its later parameter.
    for (index, left) in accesses.iter().enumerate() {
        for right in accesses.iter().skip(index + 1) {
            // Bevy accepts read/read pairs and accesses deferred to separate ParamSet members.
            let same_resource = left.resource == right.resource;
            let has_mutable_access = left.is_mutable || right.is_mutable;
            let is_deferred_to_separate_members = left.param_sets.iter().any(|left_scope| {
                right.param_sets.iter().any(|right_scope| {
                    left_scope.set_id == right_scope.set_id
                        && left_scope.member_index != right_scope.member_index
                })
            });
            let has_existing_diagnostic = conflicts
                .iter()
                .any(|conflict| conflict.system == system && conflict.span == right.span);
            if !same_resource
                || !has_mutable_access
                || is_deferred_to_separate_members
                || has_existing_diagnostic
            {
                continue;
            }
            // Keep one diagnostic for each system and later parameter span.
            conflicts.push(PendingConflict {
                system,
                span: right.span,
            });
        }
    }
}

#[cfg(test)]
/// Runtime checks for Bevy's resource access initialization contract.
mod tests {
    use bevy::ecs::{
        prelude::*,
        system::{SystemParam, SystemState},
    };
    use bevy::reflect::Reflect;

    /// A send resource used by access tests.
    #[derive(Resource, Clone, Copy, Debug, Default, Reflect)]
    struct State;

    /// A distinct send resource used to prove different-type access is valid.
    #[derive(Resource, Clone, Copy, Debug, Default, Reflect)]
    struct Other;

    /// A non-send resource used by access tests.
    #[expect(dead_code, reason = "Bevy requires a local non-send resource type")]
    struct ThreadState(std::rc::Rc<()>);

    /// A shared resource parameter with a static system lifetime.
    type Read<T> = Res<'static, T>;
    /// A mutable resource parameter with a static system lifetime.
    type Write<T> = ResMut<'static, T>;
    /// A shared non-send resource parameter with a static system lifetime.
    type LocalRead<T> = NonSend<'static, T>;
    /// A mutable non-send resource parameter with a static system lifetime.
    type LocalWrite<T> = NonSendMut<'static, T>;
    /// A `ParamSet` with static system lifetimes.
    type Parameters<P> = ParamSet<'static, 'static, P>;

    /// Confirm Bevy rejects the resource conflicts covered by this lint.
    #[test]
    fn conflicting_resource_access_fails_system_initialization() {
        // Mutable/shared and mutable/mutable pairs must fail Bevy initialization.
        expect_initialization_panic::<(Write<State>, Read<State>)>();
        // Tuple nesting must retain simultaneous access within a ParamSet member.
        expect_initialization_panic::<(Write<State>, Write<State>)>();
        expect_initialization_panic::<((Write<State>,), Read<State>)>();
        expect_initialization_panic::<(Write<State>, LocalRead<State>)>();
        expect_initialization_panic::<(LocalWrite<ThreadState>, LocalRead<ThreadState>)>();
        expect_initialization_panic::<(Parameters<(Write<State>, Read<State>)>, Read<State>)>();
        expect_initialization_panic::<Parameters<((Write<State>, Read<State>),)>>();
    }

    /// Confirm shared access and internal `ParamSet` alternatives initialize in Bevy.
    #[test]
    fn valid_resource_access_initializes() {
        // Shared access, distinct types, and separate ParamSet members are valid.
        initialize::<(Read<State>, Read<State>)>();
        initialize::<(LocalRead<ThreadState>, LocalRead<ThreadState>)>();
        initialize::<(Write<State>, Read<Other>)>();
        initialize::<Parameters<(Write<State>, Read<State>)>>();
        initialize::<Parameters<Vec<Write<State>>>>();
    }

    /// Initialize one supported Bevy system parameter in a fresh world.
    fn initialize<P: SystemParam + 'static>() {
        let mut world = World::new();
        let _state = SystemState::<P>::new(&mut world);
    }

    /// Assert that Bevy rejects one supported conflicting system parameter with B0002.
    fn expect_initialization_panic<P: SystemParam + 'static>() {
        // Capture Bevy's initialization panic and inspect the diagnostic code.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            initialize::<P>();
        }));
        // Require the resource-specific error so unrelated panics cannot satisfy this proof.
        let Err(payload) = result else {
            panic!("Bevy accepted a resource access pair that should conflict");
        };
        let payload = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or_default();
        assert!(
            payload.contains("error[B0002]"),
            "unexpected panic: {payload}"
        );
    }
}

/// Run the lint's registered-system example UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
