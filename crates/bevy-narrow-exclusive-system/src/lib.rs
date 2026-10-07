#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks narrow exclusive Bevy systems.
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
use rustc_span::{Span, def_id::LocalDefId};

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub BEVY_NARROW_EXCLUSIVE_SYSTEM,
    Warn,
    "an exclusive Bevy system only uses narrow typed world access",
    BevyNarrowExclusiveSystem,
    BevyNarrowExclusiveSystem::default()
}

/// One candidate exclusive system and its world-parameter span.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    /// Local free-function definition.
    system: LocalDefId,
    /// Span of the `&mut World` parameter type.
    parameter_span: Span,
}

/// Stateful pass that requires direct schedule registration.
///
/// The pass retains candidate function spans and direct registrations while
/// traversing one crate so it can report only semantically proven cases.
#[derive(Debug, Default)]
pub struct BevyNarrowExclusiveSystem {
    /// Functions whose world use is narrow enough for normal parameters.
    candidates: Vec<Candidate>,
    /// Direct `App::add_systems` registrations.
    registrations: Vec<bevy_support::RegisteredSystems>,
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyNarrowExclusiveSystem {
    /// Record narrow exclusive free functions.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _: Span,
        local_def_id: LocalDefId,
    ) {
        // Retain each narrow world parameter until registration evidence is available.
        let indexes =
            bevy_support::narrow_exclusive_system_parameters(cx, kind, body, local_def_id);
        self.candidates
            .extend(
                bevy_support::parameter_spans(declaration, indexes).map(|parameter_span| {
                    Candidate {
                        system: local_def_id,
                        parameter_span,
                    }
                }),
            );
    }

    /// Record direct systems registered through `App::add_systems`.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        if let Some(registration) = bevy_support::directly_registered_systems(cx, expr) {
            self.registrations.push(registration);
        }
    }

    /// Emit only for candidates registered as systems.
    fn check_crate_post(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        // Report candidates only when a direct system registration proves their role.
        for candidate in &self.candidates {
            if !self
                .registrations
                .iter()
                .any(|registration| registration.systems.contains(&candidate.system))
            {
                continue;
            }
            cx.emit_span_lint(
                BEVY_NARROW_EXCLUSIVE_SYSTEM,
                candidate.parameter_span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("this exclusive world access can use normal system parameters")
                        .help("replace &mut World with typed Query, Res, ResMut, or entity parameters");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
