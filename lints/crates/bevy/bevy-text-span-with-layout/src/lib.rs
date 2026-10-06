#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks direct Bevy bundles that put text layout on a text span.
//!
//! The lint resolves `TextSpan`, `TextLayout`, and Bevy bundle methods through
//! the compiler before reporting a local tuple bundle.
//! Bevy's text processor warns when a changed `TextSpan` entity also has
//! `TextLayout`, which belongs on a root `Text` or `Text2d` entity. The lint
//! reports only component combinations visible in one recognized bundle.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LintContext as _};

use bevy_support as _;

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_TEXT_SPAN_WITH_LAYOUT,
    Warn,
    "a Bevy bundle puts root-only text layout on a text span",
    BevyTextSpanWithLayout
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyTextSpanWithLayout {
    /// Check direct Bevy bundle arguments for a text span and its root-only layout.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Ignore methods whose receiver and argument do not form a recognized Bevy bundle call.
        let Some(bundle) = bevy_bundle_argument(cx, expr) else {
            return;
        };

        // Resolve component identities independently so same-named local types remain clean.
        let items = tuple_items(bundle);
        let has_span = has_component(cx, &items, "TextSpan");
        let has_layout = has_component(cx, &items, "TextLayout");

        // Report only when both root-layout and span types occur in this bundle expression.
        if has_span && has_layout {
            cx.emit_span_lint(
                BEVY_TEXT_SPAN_WITH_LAYOUT,
                bundle.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("this bundle puts `TextLayout` on a `TextSpan` entity")
                        .help("put `TextLayout` on the root entity that has `Text` or `Text2d`");
                }),
            );
        }
    }
}

/// Return a direct bundle argument from a known Bevy spawn or insert method.
fn bevy_bundle_argument<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Inspect only method calls because Bevy's bundle APIs take their bundle as argument zero.
    let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let bundle = arguments.first()?;
    let method = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;

    // Accept only the Bevy ECS methods whose first argument is one bundle.
    if cx.tcx.crate_name(method.krate).as_str() != "bevy_ecs"
        || !matches!(
            cx.tcx.item_name(method).as_str(),
            "spawn" | "insert" | "insert_if_new"
        )
    {
        return None;
    }

    // Restrict method names to ECS bundle operations before inspecting receiver ownership.
    let receiver_is_bundle_owner = [
        "Commands",
        "World",
        "EntityCommands",
        "EntityWorldMut",
        "RelatedSpawner",
        "RelatedSpawnerCommands",
    ]
    .into_iter()
    .any(|type_name| bevy_support::expression_has_type(cx, receiver, "bevy_ecs", type_name));
    receiver_is_bundle_owner.then_some(bundle)
}

/// Return every direct tuple item, flattening nested tuple bundles.
fn tuple_items<'tcx>(expr: &'tcx Expr<'tcx>) -> Vec<&'tcx Expr<'tcx>> {
    if let ExprKind::Tup(items) = expr.kind {
        return items.iter().flat_map(tuple_items).collect();
    }
    vec![expr]
}

/// Return whether a tuple item resolves to one Bevy text component type.
fn has_component(cx: &LateContext<'_>, items: &[&Expr<'_>], type_name: &str) -> bool {
    items
        .iter()
        .any(|item| bevy_support::expression_has_type(cx, item, "bevy_text", type_name))
}

/// Run the UI test suite for this lint.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    //! Check the local text component combination without loading fonts or a renderer.

    use bevy::ecs::{bundle::Bundle, entity::Entity, world::World};
    use bevy::text::{TextLayout, TextSpan};

    /// Store a test bundle without exposing its tuple syntax to the lint under test.
    fn spawn_for_test<B: Bundle>(world: &mut World, bundle: B) -> Entity {
        world.spawn(bundle).id()
    }

    /// Verify that a text span and layout can occupy one Bevy entity.
    #[test]
    fn world_stores_text_span_and_layout_together() {
        let mut world = World::new();
        let entity = spawn_for_test(&mut world, (TextSpan::new("span"), TextLayout::default()));

        assert!(world.get::<TextSpan>(entity).is_some());
        assert!(world.get::<TextLayout>(entity).is_some());
    }
}
