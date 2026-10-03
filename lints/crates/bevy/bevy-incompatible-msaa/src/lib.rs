#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks direct Bevy bundles for render components incompatible with enabled MSAA.
//!
//! The lint resolves Bevy types and explicit `Msaa` variants through the
//! compiler before reporting known ECS bundle methods.
//! Deferred rendering, SSAO, and OIT use different runtime queries and
//! consequences, so the lint checks their component filters independently.
//! It reports only explicit sample variants in a recognized bundle expression.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;
use rustc_hir::{
    Expr, ExprKind,
    def::{CtorOf, DefKind, Res},
};
use rustc_lint::{LateContext, LintContext as _};

use bevy_support as _;

#[cfg(test)]
use {bevy_camera as _, bevy_core_pipeline as _, bevy_ecs as _, bevy_pbr as _, bevy_render as _};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_INCOMPATIBLE_MSAA,
    Warn,
    "a Bevy bundle combines enabled MSAA with an incompatible render component",
    BevyIncompatibleMsaa
}

/// One runtime consequence of combining a render feature with enabled MSAA.
#[derive(Clone, Copy)]
enum Incompatibility {
    /// Deferred rendering disables MSAA on the camera.
    DeferredRendering,
    /// SSAO extraction stops when the camera has enabled MSAA.
    Ssao,
    /// OIT panics when the settings entity uses more than one sample.
    Oit,
}

impl Incompatibility {
    /// Return the diagnostic that states this feature's consequence.
    const fn message(self) -> &'static str {
        match self {
            Self::DeferredRendering => {
                "this bundle combines enabled MSAA with deferred rendering; the active pipeline disables MSAA"
            }
            Self::Ssao => {
                "this bundle combines enabled MSAA with SSAO; the active plugin skips SSAO extraction"
            }
            Self::Oit => {
                "this bundle combines enabled MSAA with OIT; the active plugin panics when it runs"
            }
        }
    }
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyIncompatibleMsaa {
    /// Check direct Bevy bundle arguments for explicit incompatible settings.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Ignore calls whose resolved method and receiver do not accept one direct Bevy bundle.
        let Some(bundle) = bevy_bundle_argument(cx, expr) else {
            return;
        };

        // Keep each render feature's consequence separate because Bevy applies different checks.
        for incompatibility in bundle_incompatibilities(cx, bundle) {
            cx.emit_span_lint(
                BEVY_INCOMPATIBLE_MSAA,
                bundle.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic =
                        diagnostic.primary_message(incompatibility.message());
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
    // Inspect only method calls because Bevy's bundle APIs pass the bundle as argument zero.
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

    // Check the receiver after method identity because several unrelated ECS methods are named spawn.
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

/// Return the incompatibilities proven by one tuple bundle.
fn bundle_incompatibilities(cx: &LateContext<'_>, bundle: &Expr<'_>) -> Vec<Incompatibility> {
    let items = tuple_items(bundle);
    if !items.iter().any(|item| is_enabled_msaa_variant(cx, item)) {
        return Vec::new();
    }

    // Only evaluate render-feature filters after this bundle proves enabled MSAA.
    // Match the filter used by each Bevy runtime check independently.
    let has_camera = has_component(cx, &items, "bevy_camera", "Camera")
        || has_component(cx, &items, "bevy_camera", "Camera2d")
        || has_component(cx, &items, "bevy_camera", "Camera3d");
    let has_deferred = has_component(cx, &items, "bevy_core_pipeline", "DeferredPrepass");
    let has_ssao_camera = has_component(cx, &items, "bevy_camera", "Camera3d");
    let has_ssao = has_component(cx, &items, "bevy_pbr", "ScreenSpaceAmbientOcclusion");
    let has_oit = has_component(
        cx,
        &items,
        "bevy_core_pipeline",
        "OrderIndependentTransparencySettings",
    );

    // Keep one report per incompatible feature so each diagnostic gives its runtime result.
    let mut incompatibilities = Vec::new();
    if has_camera && has_deferred {
        incompatibilities.push(Incompatibility::DeferredRendering);
    }
    if has_ssao_camera && has_ssao {
        incompatibilities.push(Incompatibility::Ssao);
    }
    // Bevy 0.19.0's OIT check filters only for its settings component.
    if has_oit {
        incompatibilities.push(Incompatibility::Oit);
    }
    incompatibilities
}

/// Return whether a tuple item resolves to one Bevy component type.
fn has_component(
    cx: &LateContext<'_>,
    items: &[&Expr<'_>],
    crate_name: &str,
    type_name: &str,
) -> bool {
    items
        .iter()
        .any(|item| bevy_support::expression_has_type(cx, item, crate_name, type_name))
}

/// Return whether an expression names an explicit enabled Bevy MSAA variant.
fn is_enabled_msaa_variant(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Reject expressions with another resolved type before inspecting path resolution.
    if !bevy_support::expression_has_type(cx, expr, "bevy_render", "Msaa") {
        return false;
    }
    let ExprKind::Path(path) = expr.kind else {
        return false;
    };

    // A typed path may still resolve to a constant or local instead of an enum variant.
    let Res::Def(kind, definition) = cx.typeck_results().qpath_res(&path, expr.hir_id) else {
        return false;
    };

    // Accept only enum-value definitions; constants and local bindings need value evaluation.
    let variant = if kind == DefKind::Variant {
        definition
    } else if let DefKind::Ctor(CtorOf::Variant, _) = kind {
        cx.tcx.parent(definition)
    } else {
        return false;
    };
    matches!(
        cx.tcx.item_name(variant).as_str(),
        "Sample2" | "Sample4" | "Sample8"
    )
}

/// Run the UI test suite for this lint.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    //! Check Bevy's component insertion contract without starting a renderer.

    use bevy_camera::{Camera, Camera3d};
    use bevy_core_pipeline::{
        oit::OrderIndependentTransparencySettings,
        prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
    };
    use bevy_ecs::{bundle::Bundle, entity::Entity, world::World};
    use bevy_pbr::ScreenSpaceAmbientOcclusion;
    use bevy_render::view::Msaa;

    /// Insert a test bundle through the generic seam.
    ///
    /// This keeps the test outside the lint's direct-tuple matcher.
    fn spawn_for_test<B: Bundle>(world: &mut World, bundle: B) -> Entity {
        world.spawn(bundle).id()
    }

    /// Verify Bevy's deferred-rendering components and required camera marker.
    #[test]
    fn deferred_rendering_camera_has_the_required_marker() {
        let mut world = World::new();

        // Spawn through the generic seam because this crate's lint checks direct bundle tuples.
        let deferred = spawn_for_test(
            &mut world,
            (Camera3d::default(), Msaa::Sample2, DeferredPrepass),
        );
        assert_eq!(world.get::<Msaa>(deferred), Some(&Msaa::Sample2));
        assert!(world.get::<Camera>(deferred).is_some());
    }

    /// Verify SSAO inserts the depth and normal prepasses required by extraction.
    #[test]
    fn ssao_adds_its_required_prepasses() {
        let mut world = World::new();

        // Spawn through the generic seam so the lint does not inspect this test tuple.
        let ssao = spawn_for_test(
            &mut world,
            (
                Camera3d::default(),
                Msaa::Sample4,
                ScreenSpaceAmbientOcclusion::default(),
            ),
        );
        assert_eq!(world.get::<Msaa>(ssao), Some(&Msaa::Sample4));
        assert!(world.get::<DepthPrepass>(ssao).is_some());
        assert!(world.get::<NormalPrepass>(ssao).is_some());
    }

    /// Verify OIT settings can be stored without a camera entity component.
    #[test]
    fn oit_settings_do_not_require_a_camera_component() {
        let mut world = World::new();

        // This boundary matches Bevy 0.19.0's OIT query, which filters on settings only.
        let oit = spawn_for_test(
            &mut world,
            (
                Msaa::Sample8,
                OrderIndependentTransparencySettings::default(),
            ),
        );
        assert_eq!(world.get::<Msaa>(oit), Some(&Msaa::Sample8));
        assert!(world.get::<Camera>(oit).is_none());
    }
}
