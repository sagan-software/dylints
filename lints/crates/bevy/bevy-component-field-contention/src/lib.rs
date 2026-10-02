#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks disjoint field writes through one Bevy component.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use dylint_linting as _;
use rustc_lint::LintContext as _;
use rustc_span::def_id::LocalDefId;

#[cfg(test)]
use {bevy_app as _, bevy_ecs as _};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub BEVY_COMPONENT_FIELD_CONTENTION,
    Warn,
    "systems contend on one component while writing disjoint fields",
    BevyComponentFieldContention,
    BevyComponentFieldContention::default()
}

/// Field access recorded for one local system function.
#[derive(Clone, Debug)]
struct SystemAccess {
    /// Local system function.
    system: LocalDefId,
    /// Mutable component field access.
    access: bevy_support::ComponentFieldAccess,
}

/// Stateful pass that correlates systems with their schedule registrations.
///
/// The pass retains resolved accesses and direct registrations while traversing
/// one crate so it can report only semantically proven contention.
#[derive(Debug, Default)]
pub struct BevyComponentFieldContention {
    /// Mutable field accesses recorded by system function.
    accesses: Vec<SystemAccess>,
    /// Direct `App::add_systems` registrations.
    registrations: Vec<bevy_support::RegisteredSystems>,
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyComponentFieldContention {
    /// Record field-level access for direct free functions.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _: rustc_span::Span,
        local_def_id: LocalDefId,
    ) {
        let Some(access) =
            bevy_support::mutable_component_field_access(cx, kind, body, local_def_id)
        else {
            return;
        };
        self.accesses.push(SystemAccess {
            system: local_def_id,
            access,
        });
    }

    /// Record direct systems and their schedule-label type.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        if let Some(registration) = bevy_support::directly_registered_systems(cx, expr) {
            self.registrations.push(registration);
        }
    }

    /// Emit once for each component with disjoint same-schedule writers.
    fn check_crate_post(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        // Compare each system pair once and deduplicate affected components.
        let mut components = Vec::new();
        for (index, left) in self.accesses.iter().enumerate() {
            for right in self.accesses.iter().skip(index + 1) {
                if left.access.component != right.access.component
                    || !fields_are_disjoint(&left.access.fields, &right.access.fields)
                    || !self.is_schedule_shared(left.system, right.system)
                    || components.contains(&left.access.component)
                {
                    continue;
                }
                // Retain each affected component once across all system pairs.
                components.push(left.access.component);
            }
        }
        // Emit one component-level diagnostic after all registration evidence is known.
        for component in components {
            cx.emit_span_lint(
                BEVY_COMPONENT_FIELD_CONTENTION,
                cx.tcx.def_span(component),
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(
                            "systems in one schedule write disjoint fields of this component",
                        )
                        .help("split independently written fields into separate components");
                }),
            );
        }
    }
}

impl BevyComponentFieldContention {
    /// Return whether both functions are registered under one schedule-label type.
    fn is_schedule_shared(&self, left: LocalDefId, right: LocalDefId) -> bool {
        self.registrations.iter().any(|left_registration| {
            left_registration.systems.contains(&left)
                && self.registrations.iter().any(|right_registration| {
                    right_registration.schedule == left_registration.schedule
                        && right_registration.systems.contains(&right)
                })
        })
    }
}

/// Return whether two nonempty field sets have no member in common.
fn fields_are_disjoint(left: &[rustc_span::Symbol], right: &[rustc_span::Symbol]) -> bool {
    !left.is_empty() && !right.is_empty() && left.iter().all(|field| !right.contains(field))
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
