#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Detects frame-edge reads from standard Bevy button resources in fixed systems.
//!
//! The pass resolves exact Bevy `App`, `FixedUpdate`, and `ButtonInput<T>` APIs,
//! then joins edge reads to local function registrations. It recognizes direct
//! and tuple-wrapped `Res` or `ResMut` parameters, skips nested closure bodies
//! and held-state queries, and warns on `just_pressed` or `just_released`.
//!
//! It assumes frame-based input updates. It does not trace through
//! `Option<Res<_>>`, `ParamSet<_>`, custom system-parameter wrappers, locals,
//! helpers, or user-maintained input buffers.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use dylint_linting as _;
use rustc_hir::intravisit::Visitor;
use rustc_lint::LintContext as _;
use rustc_span::{Span, def_id::LocalDefId};

#[cfg(test)]
mod fixed_schedule_behavior {
    use bevy_app::FixedUpdate;
    use bevy_ecs::{
        resource::Resource,
        schedule::Schedule,
        system::{Res, ResMut},
        world::World,
    };
    use bevy_input::{ButtonInput, keyboard::KeyCode};
    use bevy_reflect::Reflect;

    /// Counts edge-driven actions performed by one fixed update.
    #[derive(Default, Reflect, Resource)]
    struct ActionCount {
        /// Number of edge-driven actions that have run.
        count: u32,
    }

    /// Counts edge-driven actions without retaining frame-edge state.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Bevy system parameters are passed by value"
    )]
    fn unbuffered_jump(keys: Res<'_, ButtonInput<KeyCode>>, mut actions: ResMut<'_, ActionCount>) {
        if keys.just_pressed(KeyCode::Space) {
            actions.count += 1;
        }
    }

    /// Stores a frame edge until a fixed tick consumes it.
    #[derive(Default, Reflect, Resource)]
    struct BufferedJump {
        /// Whether one jump edge is waiting for a fixed tick.
        has_pending: bool,
    }

    /// Copies a frame edge into the fixed-step buffer.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Bevy system parameters are passed by value"
    )]
    fn capture_jump(keys: Res<'_, ButtonInput<KeyCode>>, mut buffered: ResMut<'_, BufferedJump>) {
        buffered.has_pending |= keys.just_pressed(KeyCode::Space);
    }

    /// Consumes each buffered edge once.
    fn consume_jump(mut buffered: ResMut<'_, BufferedJump>, mut actions: ResMut<'_, ActionCount>) {
        if core::mem::take(&mut buffered.has_pending) {
            actions.count += 1;
        }
    }

    /// Shows that one frame edge can drive a fixed system more than once.
    #[test]
    fn repeated_fixed_ticks_repeat_an_unbuffered_edge_action() {
        // Set one frame edge before two fixed ticks can sample it.
        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Space);
        world.insert_resource(keys);
        world.insert_resource(ActionCount::default());

        let mut fixed = Schedule::new(FixedUpdate);
        let _configured = fixed.add_systems(unbuffered_jump);
        fixed.run(&mut world);
        fixed.run(&mut world);

        // Leave ButtonInput transitions intact between ticks to expose repetition.
        let action_count = world
            .get_resource::<ActionCount>()
            .map(|actions| actions.count);
        // The optional lookup keeps a missing test resource observable as failure.
        assert_eq!(action_count, Some(2));
    }

    /// Shows that a zero-tick frame can lose an edge before the next fixed tick.
    #[test]
    fn skipped_fixed_tick_loses_an_unbuffered_edge_at_the_next_input_update() {
        // Set an edge in one frame, then model the next frame's input clear.
        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Space);
        world.insert_resource(keys);
        world.insert_resource(ActionCount::default());

        let mut fixed = Schedule::new(FixedUpdate);
        let _configured = fixed.add_systems(unbuffered_jump);
        let is_input_cleared = world
            .get_resource_mut::<ButtonInput<KeyCode>>()
            .map(|mut keys| keys.clear())
            .is_some();
        assert!(is_input_cleared);
        fixed.run(&mut world);

        // The fixed schedule runs only after the transition has been cleared.
        let action_count = world
            .get_resource::<ActionCount>()
            .map(|actions| actions.count);
        // The optional lookup keeps a missing test resource observable as failure.
        assert_eq!(action_count, Some(0));
    }

    /// Shows that an input buffer preserves a skipped edge and consumes it once.
    #[test]
    fn buffered_edge_survives_zero_ticks_and_drives_one_later_action() {
        // Capture before the fixed loop, which may execute no schedule this frame.
        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Space);
        world.insert_resource(keys);
        world.insert_resource(BufferedJump::default());
        world.insert_resource(ActionCount::default());

        let mut before_fixed = Schedule::default();
        let _capture_configured = before_fixed.add_systems(capture_jump);
        let mut fixed = Schedule::new(FixedUpdate);
        let _consume_configured = fixed.add_systems(consume_jump);

        before_fixed.run(&mut world);
        // Clear the frame edge before the next capture and any later fixed tick.
        let is_input_cleared = world
            .get_resource_mut::<ButtonInput<KeyCode>>()
            .map(|mut keys| keys.clear())
            .is_some();
        assert!(is_input_cleared);
        // Capturing the now-empty input must preserve the previously buffered edge.
        before_fixed.run(&mut world);
        // The first fixed tick consumes the edge; a second tick sees an empty buffer.
        fixed.run(&mut world);
        fixed.run(&mut world);

        let action_count = world
            .get_resource::<ActionCount>()
            .map(|actions| actions.count);
        assert_eq!(action_count, Some(1));
    }
}

/// One direct fixed-update system and its frame-edge method calls.
#[derive(Clone, Debug)]
struct FixedInputUse {
    /// The local function registered in `FixedUpdate`.
    system: LocalDefId,
    /// Calls that read frame-only button transitions.
    spans: Vec<Span>,
}

/// Stateful pass that joins resolved button-edge reads to direct schedule
/// registrations.
///
/// It emits warnings only when the same local function has a recognized
/// standard `ButtonInput<T>` resource parameter and a `FixedUpdate` registration.
#[derive(Debug, Default)]
pub struct BevyFrameButtonEdgeInFixedUpdate {
    /// Functions that read frame-only edges from a standard button resource.
    input_uses: Vec<FixedInputUse>,
    /// Functions directly registered through `App::add_systems(FixedUpdate, ..)`.
    fixed_systems: Vec<LocalDefId>,
}

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub BEVY_FRAME_BUTTON_EDGE_IN_FIXED_UPDATE,
    Warn,
    "a frame-only button edge is read from a fixed-update system",
    BevyFrameButtonEdgeInFixedUpdate,
    BevyFrameButtonEdgeInFixedUpdate::default()
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyFrameButtonEdgeInFixedUpdate {
    /// Record direct functions that receive Bevy button resources and read an edge.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _: Span,
        local_def_id: LocalDefId,
    ) {
        // The group resolves direct free-function registrations; skip closure signatures.
        if !matches!(kind, rustc_hir::intravisit::FnKind::ItemFn(..)) {
            return;
        }
        // Scan direct and tuple-wrapped standard input resource parameters.
        let has_button_input_resource = cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .inputs()
            .skip_binder()
            .iter()
            .any(|input| is_button_input_system_param(cx, *input));
        if !has_button_input_resource {
            return;
        }

        // Resolve only ButtonInput's edge methods inside this function body.
        let mut visitor = ButtonEdgeVisitor {
            cx,
            spans: Vec::new(),
        };
        visitor.visit_expr(body.value);
        if !visitor.spans.is_empty() {
            self.input_uses.push(FixedInputUse {
                system: local_def_id,
                spans: visitor.spans,
            });
        }
    }

    /// Record direct Bevy `FixedUpdate` registrations with the exact schedule type.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        // Resolve the Bevy application method before interpreting its schedule argument.
        let Some(call) = bevy_support::bevy_method_call(cx, expr, "bevy_app", "App", "add_systems")
            .or_else(|| {
                bevy_support::bevy_method_call(cx, expr, "bevy_app", "SubApp", "add_systems")
            })
        else {
            return;
        };
        let Some(schedule) = call.arguments.first() else {
            return;
        };
        if !bevy_support::expression_has_type(cx, schedule, "bevy_app", "FixedUpdate") {
            return;
        }

        // Reuse the group's local-function resolver after proving the schedule identity.
        if let Some(registration) = bevy_support::directly_registered_systems(cx, expr) {
            self.fixed_systems.extend(registration.systems);
        }
    }

    /// Emit one diagnostic at each edge call whose function is directly fixed-scheduled.
    fn check_crate_post(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        // Join the independently collected read and registration facts by local function.
        for input_use in &self.input_uses {
            if !self.fixed_systems.contains(&input_use.system) {
                continue;
            }
            for span in &input_use.spans {
                cx.emit_span_lint(
                    BEVY_FRAME_BUTTON_EDGE_IN_FIXED_UPDATE,
                    *span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic = diagnostic
                            .primary_message(
                                "this button edge lasts one frame but is read in `FixedUpdate`",
                            )
                            .help("capture the edge before the fixed loop and pass it to fixed-step systems");
                    }),
                );
            }
        }
    }
}

/// Return whether a parameter or tuple contains `Res` or `ResMut` of `ButtonInput<T>`.
fn is_button_input_system_param(
    cx: &rustc_lint::LateContext<'_>,
    parameter: rustc_middle::ty::Ty<'_>,
) -> bool {
    // Bevy supports tuple SystemParams, so inspect their members recursively.
    let parameter = parameter.peel_refs();
    if let rustc_middle::ty::TyKind::Tuple(parameters) = parameter.kind() {
        return parameters
            .iter()
            .any(|parameter| is_button_input_system_param(cx, parameter));
    }
    // Accept only exact Bevy resource parameter wrappers around ButtonInput.
    let rustc_middle::ty::TyKind::Adt(wrapper, arguments) = parameter.kind() else {
        return false;
    };
    let wrapper_id = wrapper.did();
    let is_resource_param = cx.tcx.crate_name(wrapper_id.krate).as_str() == "bevy_ecs"
        && matches!(cx.tcx.item_name(wrapper_id).as_str(), "Res" | "ResMut");
    is_resource_param
        && arguments
            .types()
            .any(|argument| bevy_support::type_is_named(cx, argument, "bevy_input", "ButtonInput"))
}

/// Collect frame-edge method spans from exact Bevy `ButtonInput` receivers.
struct ButtonEdgeVisitor<'cx, 'tcx> {
    /// Type context used to resolve each method to its inherent Bevy implementation.
    cx: &'cx rustc_lint::LateContext<'tcx>,
    /// Resolved frame-edge calls found in the system body.
    spans: Vec<Span>,
}

impl<'tcx> Visitor<'tcx> for ButtonEdgeVisitor<'_, 'tcx> {
    /// Skip nested closure bodies because this pass only classifies the named system body.
    fn visit_nested_body(&mut self, _body_id: rustc_hir::BodyId) {}

    /// Record `just_pressed` and `just_released` calls before visiting their arguments.
    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if is_button_edge_method(self.cx, expr)
            && let rustc_hir::ExprKind::MethodCall(segment, _, _, _) = expr.kind
        {
            self.spans.push(segment.ident.span);
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// Return whether a method call is an inherent edge query on Bevy `ButtonInput`.
fn is_button_edge_method(cx: &rustc_lint::LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
    // Restrict analysis to the two frame-transition method names.
    let rustc_hir::ExprKind::MethodCall(segment, _, _, _) = expr.kind else {
        return false;
    };
    if !matches!(
        segment.ident.name.as_str(),
        "just_pressed" | "just_released"
    ) {
        return false;
    }
    let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    // Resolve the declaring impl so same-named application methods stay clean.
    cx.tcx
        .inherent_impl_of_assoc(method)
        .is_some_and(|implementation| {
            bevy_support::type_is_named(
                cx,
                cx.tcx
                    .type_of(implementation)
                    .instantiate_identity()
                    .skip_norm_wip(),
                "bevy_input",
                "ButtonInput",
            )
        })
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
