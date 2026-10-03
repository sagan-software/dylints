#![feature(rustc_private)]

//! Rust review lints for Bevy API misuse.
//!
//! This group resolves scheduling, component, query, world, resource, and
//! bundle patterns through one Dylint entry point. Each constituent lint owns
//! its semantic checks, UI fixture, configuration, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register every Bevy lint in deterministic subsystem order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to focused registration helpers for each subsystem.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = bevy::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register text rasterization and render configuration checks.
    bevy_continuous_font_size::register_lints(sess, lint_store);
    bevy_continuous_ui_scale::register_lints(sess, lint_store);
    bevy_invalid_constant_font_size::register_lints(sess, lint_store);
    bevy_incompatible_msaa::register_lints(sess, lint_store);
    bevy_text_span_with_layout::register_lints(sess, lint_store);
    // Register asset setup and lazy message consumption checks.
    bevy_asset_source_after_asset_plugin::register_lints(sess, lint_store);
    bevy_load_builder_replaced_guard::register_lints(sess, lint_store);
    bevy_message_presence_without_consumption::register_lints(sess, lint_store);
    // Register parameter access and fixed-loop input checks.
    bevy_conflicting_resource_params::register_lints(sess, lint_store);
    bevy_conflicting_query_params::register_lints(sess, lint_store);
    bevy_mouse_displacement_times_delta::register_lints(sess, lint_store);
    bevy_frame_button_edge_in_fixed_update::register_lints(sess, lint_store);
    register_bevy_schedule_lints(sess, lint_store);
    register_bevy_query_lints(sess, lint_store);
    register_bevy_world_lints(sess, lint_store);
}

/// Register Bevy scheduling, dependency, and component-size policies.
fn register_bevy_schedule_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Register scheduling and dependency policies before message and size checks.
    bevy_borrowed_reborrowable::register_lints(sess, lint_store);
    bevy_camera_modification_in_fixed_update::register_lints(sess, lint_store);
    bevy_children_mutation_query::register_lints(sess, lint_store);
    bevy_component_field_contention::register_lints(sess, lint_store);
    bevy_disallow_fixed_update_schedule::register_lints(sess, lint_store);
    // Continue with scheduling and duplicate-registration policies.
    bevy_disallow_update_schedule::register_lints(sess, lint_store);
    bevy_duplicate_dependencies::register_lints(sess, lint_store);
    bevy_duplicate_plugin_addition::register_lints(sess, lint_store);
    bevy_global_transform_mutation_query::register_lints(sess, lint_store);
    bevy_inherited_visibility_mutation_query::register_lints(sess, lint_store);
    // Register message, update, and component-size policies next.
    bevy_insert_message_resource::register_lints(sess, lint_store);
    bevy_iter_current_update_messages::register_lints(sess, lint_store);
    bevy_large_component::register_lints(sess, lint_store);
    bevy_large_component_change_filter::register_lints(sess, lint_store);
    bevy_main_return_without_app_exit::register_lints(sess, lint_store);
}

/// Register Bevy component-trait, reflection, and query-shape policies.
fn register_bevy_query_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Continue with component traits and reflection before query access checks.
    bevy_missing_clone_for_unit_component::register_lints(sess, lint_store);
    bevy_missing_copy_for_unit_component::register_lints(sess, lint_store);
    bevy_missing_default_for_unit_component::register_lints(sess, lint_store);
    bevy_missing_reflect::register_lints(sess, lint_store);
    bevy_narrow_exclusive_system::register_lints(sess, lint_store);
    // Register query-shape and time policies next.
    bevy_partially_used_query_data::register_lints(sess, lint_store);
    bevy_presence_only_query_fetch::register_lints(sess, lint_store);
    bevy_readonly_system_access::register_lints(sess, lint_store);
    bevy_time_elapsed_secs_cast_f64::register_lints(sess, lint_store);
    bevy_unconventional_naming::register_lints(sess, lint_store);
}

/// Register Bevy world, bundle, resource, and schedule-scope policies.
fn register_bevy_world_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Finish with access breadth, bundles, resources, and world operations.
    bevy_unfiltered_entity_access_query::register_lints(sess, lint_store);
    bevy_unit_in_bundle::register_lints(sess, lint_store);
    bevy_wide_query_access::register_lints(sess, lint_store);
    bevy_world_entity::register_lints(sess, lint_store);
    bevy_world_entity_mut::register_lints(sess, lint_store);
    bevy_world_insert_batch::register_lints(sess, lint_store);
    // Register batch, non-send, and resource-access policies next.
    bevy_world_insert_batch_if_new::register_lints(sess, lint_store);
    bevy_world_non_send::register_lints(sess, lint_store);
    bevy_world_non_send_mut::register_lints(sess, lint_store);
    bevy_world_resource::register_lints(sess, lint_store);
    bevy_world_resource_mut::register_lints(sess, lint_store);
    bevy_world_resource_ref::register_lints(sess, lint_store);
    // Finish with schedule and zero-sized query policies.
    bevy_world_run_schedule::register_lints(sess, lint_store);
    bevy_world_schedule_scope::register_lints(sess, lint_store);
    bevy_zst_query::register_lints(sess, lint_store);
}
