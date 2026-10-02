#![feature(rustc_private)]
//! Rust review lints for crate-specific documented API misuse.
//!
//! This registry installs the crate-specific lint families for Axum, Bevy, Clap,
//! Insta, Reqwest, Schemars, Serde, `SQLx`, test-case, thiserror, Tokio, and
//! tracing. Each family remains in a dedicated registration helper so the
//! entry point stays deterministic and each helper remains easy to audit.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register every crate-specific lint in deterministic family order.
///
/// Dylint calls this entry point once per compilation. It forwards the compiler
/// session and lint store to each family helper, which installs constituent lints
/// without changing their individual diagnostics or configuration.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = crates::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register route and scheduling families before general crate APIs.
    register_axum_lints(sess, lint_store);
    register_bevy_lints(sess, lint_store);
    // Register command-line and snapshot families next.
    register_clap_lints(sess, lint_store);
    register_insta_lints(sess, lint_store);
    // Register network, schema, serialization, and database families.
    register_reqwest_lints(sess, lint_store);
    register_schemars_lints(sess, lint_store);
    register_serde_lints(sess, lint_store);
    register_sqlx_lints(sess, lint_store);
    strum_enum_representation::register_lints(sess, lint_store);
    // Register test, error, runtime, and tracing families last.
    register_test_case_lints(sess, lint_store);
    register_thiserror_lints(sess, lint_store);
    register_tokio_lints(sess, lint_store);
    register_tracing_lints(sess, lint_store);
}

/// Register the Axum constituent lints in route-family order.
fn register_axum_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register nesting policies before route construction policies.
    axum_nest_at_root::register_lints(sess, lint_store);
    axum_nest_service_at_root::register_lints(sess, lint_store);
    axum_nest_wildcard_path::register_lints(sess, lint_store);
    axum_route_empty_path::register_lints(sess, lint_store);
    axum_route_layer_on_empty_router::register_lints(sess, lint_store);
    // Finish with legacy capture, path, and service policies.
    axum_route_legacy_colon_capture::register_lints(sess, lint_store);
    axum_route_legacy_wildcard_capture::register_lints(sess, lint_store);
    axum_route_path_missing_slash::register_lints(sess, lint_store);
    axum_route_service_empty_path::register_lints(sess, lint_store);
    axum_route_service_router::register_lints(sess, lint_store);
}

/// Register the Clap constituent lints in parser and derive order.
fn register_clap_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register value and command-shape policies before derive checks.
    clap_allow_hyphen_values_without_num_args::register_lints(sess, lint_store);
    clap_allow_negative_numbers_without_num_args::register_lints(sess, lint_store);
    clap_command_with_arguments_option::register_lints(sess, lint_store);
    clap_derive_author_without_help_template::register_lints(sess, lint_store);
    clap_derive_bool_default_true::register_lints(sess, lint_store);
    // Continue with derive action and value-parser policies.
    clap_derive_redundant_action::register_lints(sess, lint_store);
    clap_derive_redundant_value_parser::register_lints(sess, lint_store);
    clap_derive_vec_vec_without_num_args::register_lints(sess, lint_store);
    clap_derive_verbatim_doc_comment_without_doc::register_lints(sess, lint_store);
    // Register external-command and positional-option policies next.
    // Finish with external-command and positional-option policies.
    clap_external_subcommand_parser_without_external_subcommands::register_lints(sess, lint_store);
    clap_index_on_option::register_lints(sess, lint_store);
    clap_last_option::register_lints(sess, lint_store);
    clap_multicall_no_binary_name::register_lints(sess, lint_store);
    clap_require_equals_without_num_args::register_lints(sess, lint_store);
    clap_required_conditional_conflict::register_lints(sess, lint_store);
    clap_trailing_var_arg_without_num_args::register_lints(sess, lint_store);
}

/// Register the Insta constituent lints in snapshot-policy order.
fn register_insta_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register glob, scope, and content-shape policies first.
    insta_allow_empty_glob::register_lints(sess, lint_store);
    insta_binary_snapshot_missing_extension::register_lints(sess, lint_store);
    insta_bind_to_scope_in_async::register_lints(sess, lint_store);
    insta_compact_json_snapshot::register_lints(sess, lint_store);
    insta_content_direct_match::register_lints(sess, lint_store);
    // Continue with deprecated and empty-value policies.
    insta_deprecated_assert_display::register_lints(sess, lint_store);
    insta_empty_description::register_lints(sess, lint_store);
    insta_empty_filter_pattern::register_lints(sess, lint_store);
    insta_empty_input_file::register_lints(sess, lint_store);
    insta_empty_snapshot_path::register_lints(sess, lint_store);
    insta_empty_snapshot_suffix::register_lints(sess, lint_store);
    // Register path and JSON policies before settings and loop checks.
    // Finish with path, settings, and loop policies.
    insta_glob_parent_traversal::register_lints(sess, lint_store);
    insta_json_snapshot::register_lints(sess, lint_store);
    insta_no_module_prefix::register_lints(sess, lint_store);
    insta_noop_filter::register_lints(sess, lint_store);
    insta_redundant_content_resolve_inner::register_lints(sess, lint_store);
    insta_settings_bind_future::register_lints(sess, lint_store);
    insta_settings_new::register_lints(sess, lint_store);
    insta_settings_raw_info::register_lints(sess, lint_store);
    insta_snapshot_in_loop::register_lints(sess, lint_store);
}

/// Register the Reqwest constituent lints in request and TLS order.
fn register_reqwest_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register request-loop and client-ownership policies before retries.
    reqwest_blocking_in_async::register_lints(sess, lint_store);
    reqwest_client_in_loop::register_lints(sess, lint_store);
    reqwest_client_wrapped_in_shared_pointer::register_lints(sess, lint_store);
    reqwest_cookie_provider_overridden::register_lints(sess, lint_store);
    reqwest_get_in_loop::register_lints(sess, lint_store);
    reqwest_multipart_manual_content_type::register_lints(sess, lint_store);
    // Finish with retry-budget and TLS policies.
    reqwest_retry_invalid_max_extra_load::register_lints(sess, lint_store);
    reqwest_retry_no_budget::register_lints(sess, lint_store);
    reqwest_tls_danger_invalid_certs::register_lints(sess, lint_store);
    reqwest_tls_danger_invalid_hostnames::register_lints(sess, lint_store);
}

/// Register the Schemars constituent lints in schema-policy order.
fn register_schemars_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register reference and recursion policies before serde attributes.
    schemars_json_schema_ref_return::register_lints(sess, lint_store);
    schemars_recursive_inline_schema::register_lints(sess, lint_store);
    schemars_redundant_serde_default::register_lints(sess, lint_store);
    schemars_redundant_serde_deny_unknown_fields::register_lints(sess, lint_store);
    schemars_redundant_serde_rename::register_lints(sess, lint_store);
    // Finish with rename, skip, tag, transparent, and generation policies.
    schemars_redundant_serde_rename_all::register_lints(sess, lint_store);
    schemars_redundant_serde_skip::register_lints(sess, lint_store);
    schemars_redundant_serde_tag::register_lints(sess, lint_store);
    schemars_redundant_serde_transparent::register_lints(sess, lint_store);
    schemars_schema_for_value_json_schema::register_lints(sess, lint_store);
}

/// Register the Serde constituent lints in serialization-policy order.
fn register_serde_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register borrowing, default, portability, and attribute policies first.
    serde_all_fields_default::register_lints(sess, lint_store);
    serde_borrow_redundant_str_bytes::register_lints(sess, lint_store);
    serde_cow_missing_borrow::register_lints(sess, lint_store);
    serde_deserialize_any_portability::register_lints(sess, lint_store);
    serde_expecting_style::register_lints(sess, lint_store);
    // Continue with fallback and attribute-combination policies.
    serde_fallback_missing_other::register_lints(sess, lint_store);
    serde_flatten_deny_unknown_fields::register_lints(sess, lint_store);
    serde_inert_directional_attr::register_lints(sess, lint_store);
    serde_manual_rename_all::register_lints(sess, lint_store);
    serde_serialize_str_to_string::register_lints(sess, lint_store);
    // Finish with round-trip and untagged-enum policies.
    serde_skip_serializing_roundtrip::register_lints(sess, lint_store);
    serde_skip_serializing_variant_error::register_lints(sess, lint_store);
    serde_untagged_missing_expecting::register_lints(sess, lint_store);
}

/// Register the `SQLx` constituent lints in query-safety order.
fn register_sqlx_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register query, row, and statement policies before pool and builders.
    sqlx_assert_sql_safe::register_lints(sess, lint_store);
    sqlx_empty_push_tuples::register_lints(sess, lint_store);
    sqlx_empty_push_values::register_lints(sess, lint_store);
    sqlx_panicking_row_get::register_lints(sess, lint_store);
    sqlx_panicking_statement_column::register_lints(sess, lint_store);
    sqlx_pool_connection_leak::register_lints(sess, lint_store);
    sqlx_query_builder_push_interpolation::register_lints(sess, lint_store);
    sqlx_query_builder_push_unseparated_interpolation::register_lints(sess, lint_store);
    // Finish with unchecked-macro and connection-limit policies.
    sqlx_unchecked_query_macro::register_lints(sess, lint_store);
    sqlx_zero_max_connections::register_lints(sess, lint_store);
}

/// Register the test-case constituent lints in metadata and harness order.
fn register_test_case_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register harness, descriptions, matrices, and panic policies first.
    test_case_async_without_test_harness::register_lints(sess, lint_store);
    test_case_constant_match_guard::register_lints(sess, lint_store);
    test_case_empty_description::register_lints(sess, lint_store);
    test_case_empty_ignore_reason::register_lints(sess, lint_store);
    test_case_empty_matrix::register_lints(sess, lint_store);
    // Continue with panic and ignore metadata before suite-size policies.
    test_case_empty_panic_message::register_lints(sess, lint_store);
    test_case_ignore_without_reason::register_lints(sess, lint_store);
    test_case_ignored_matrix::register_lints(sess, lint_store);
    test_case_inconclusive_modifier::register_lints(sess, lint_store);
    test_case_large_suite::register_lints(sess, lint_store);
    test_case_legacy_inconclusive_description::register_lints(sess, lint_store);
    // Continue with ignore, inconclusive, and suite-size policies.
    // Continue with precision, matrix-shape, naming, and whitespace policies.
    test_case_nonpositive_almost_precision::register_lints(sess, lint_store);
    test_case_panics_without_message::register_lints(sess, lint_store);
    test_case_single_case_matrix::register_lints(sess, lint_store);
    test_case_singleton_contains_in_order::register_lints(sess, lint_store);
    test_case_singleton_matrix_dimension::register_lints(sess, lint_store);
    // Finish with naming, whitespace, wildcard, and path policies.
    test_case_unnamed_test_case::register_lints(sess, lint_store);
    test_case_whitespace_ignore_reason::register_lints(sess, lint_store);
    test_case_whitespace_panic_message::register_lints(sess, lint_store);
    test_case_wildcard_match::register_lints(sess, lint_store);
    test_case_with_function_path::register_lints(sess, lint_store);
}

/// Register the thiserror constituent lints in field and display order.
fn register_thiserror_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register field-format and attribute policies before display contracts.
    thiserror_named_field_positional_format::register_lints(sess, lint_store);
    thiserror_no_std_path_display::register_lints(sess, lint_store);
    thiserror_raw_field_format::register_lints(sess, lint_store);
    thiserror_redundant_backtrace_attr::register_lints(sess, lint_store);
    thiserror_redundant_from_source::register_lints(sess, lint_store);
    // Continue with source-field and display-contract checks.
    thiserror_redundant_named_source::register_lints(sess, lint_store);
    thiserror_self_display_recursion::register_lints(sess, lint_store);
    thiserror_source_field_opt_out::register_lints(sess, lint_store);
    thiserror_transparent_single_source_display::register_lints(sess, lint_store);
    thiserror_tuple_format_positional_ambiguity::register_lints(sess, lint_store);
}

/// Register the Tokio constituent lints in runtime and channel order.
fn register_tokio_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register blocking, runtime, and sleep policies before task and channels.
    tokio_blocking_call_in_async::register_lints(sess, lint_store);
    tokio_handle_block_on_in_async::register_lints(sess, lint_store);
    tokio_runtime_block_on_in_async::register_lints(sess, lint_store);
    tokio_runtime_new_in_async::register_lints(sess, lint_store);
    tokio_sleep_in_loop::register_lints(sess, lint_store);
    // Finish with task, channel, interval, and thread-count policies.
    tokio_spawn_blocking_async_closure::register_lints(sess, lint_store);
    tokio_unbounded_channel::register_lints(sess, lint_store);
    tokio_zero_capacity_channel::register_lints(sess, lint_store);
    tokio_zero_duration_interval::register_lints(sess, lint_store);
    tokio_zero_runtime_thread_count::register_lints(sess, lint_store);
}

/// Register the tracing constituent lints in span and field order.
fn register_tracing_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register async scope, span lifecycle, and direct-record policies first.
    tracing_async_block_in_sync_scope::register_lints(sess, lint_store);
    tracing_await_holding_span_guard::register_lints(sess, lint_store);
    tracing_current_span_enter::register_lints(sess, lint_store);
    tracing_current_span_or_current::register_lints(sess, lint_store);
    tracing_direct_record_all::register_lints(sess, lint_store);
    // Continue with field formatting and instrumentation policies.
    tracing_format_field::register_lints(sess, lint_store);
    tracing_instrument_current_span::register_lints(sess, lint_store);
    tracing_message_interpolation::register_lints(sess, lint_store);
    tracing_none_enter::register_lints(sess, lint_store);
    tracing_none_or_current::register_lints(sess, lint_store);
    // Finish with optional-span and redundant-field policies.
    tracing_none_record::register_lints(sess, lint_store);
    tracing_redundant_field_assignment::register_lints(sess, lint_store);
    tracing_redundant_in_current_span::register_lints(sess, lint_store);
    tracing_to_string_field::register_lints(sess, lint_store);
}

/// Registers the Bevy-specific constituent lints.
fn register_bevy_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Keep Bevy registration split by subsystem so each helper remains auditable.
    register_bevy_schedule_lints(sess, lint_store);
    register_bevy_query_lints(sess, lint_store);
    register_bevy_world_lints(sess, lint_store);
}

/// Register Bevy schedule, dependency, and component policies.
fn register_bevy_schedule_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Register scheduling and dependency policies before query policies.
    bevy_borrowed_reborrowable::register_lints(sess, lint_store);
    bevy_camera_modification_in_fixed_update::register_lints(sess, lint_store);
    bevy_children_mutation_query::register_lints(sess, lint_store);
    bevy_component_field_contention::register_lints(sess, lint_store);
    bevy_disallow_fixed_update_schedule::register_lints(sess, lint_store);
    // Continue Bevy registration with scheduling and duplicate-registration policies.
    bevy_disallow_update_schedule::register_lints(sess, lint_store);
    bevy_duplicate_dependencies::register_lints(sess, lint_store);
    bevy_duplicate_plugin_addition::register_lints(sess, lint_store);
    bevy_global_transform_mutation_query::register_lints(sess, lint_store);
    bevy_inherited_visibility_mutation_query::register_lints(sess, lint_store);
    // Register Bevy message, update, and component-size policies next.
    bevy_insert_message_resource::register_lints(sess, lint_store);
    bevy_iter_current_update_messages::register_lints(sess, lint_store);
    bevy_large_component::register_lints(sess, lint_store);
    bevy_large_component_change_filter::register_lints(sess, lint_store);
    bevy_main_return_without_app_exit::register_lints(sess, lint_store);
}

/// Register Bevy component, reflection, and query-shape policies.
fn register_bevy_query_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Register component traits and reflection policies before query access.
    bevy_missing_clone_for_unit_component::register_lints(sess, lint_store);
    bevy_missing_copy_for_unit_component::register_lints(sess, lint_store);
    bevy_missing_default_for_unit_component::register_lints(sess, lint_store);
    bevy_missing_reflect::register_lints(sess, lint_store);
    bevy_narrow_exclusive_system::register_lints(sess, lint_store);
    // Register Bevy query-shape and time policies next.
    bevy_partially_used_query_data::register_lints(sess, lint_store);
    bevy_presence_only_query_fetch::register_lints(sess, lint_store);
    bevy_readonly_system_access::register_lints(sess, lint_store);
    bevy_time_elapsed_secs_cast_f64::register_lints(sess, lint_store);
    bevy_unconventional_naming::register_lints(sess, lint_store);
}

/// Register Bevy world, bundle, and schedule-scope policies.
fn register_bevy_world_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Register world access and bundle policies before batch operations.
    bevy_unfiltered_entity_access_query::register_lints(sess, lint_store);
    bevy_unit_in_bundle::register_lints(sess, lint_store);
    bevy_wide_query_access::register_lints(sess, lint_store);
    bevy_world_entity::register_lints(sess, lint_store);
    bevy_world_entity_mut::register_lints(sess, lint_store);
    bevy_world_insert_batch::register_lints(sess, lint_store);
    // Register Bevy batch, non-send, and resource-access policies next.
    bevy_world_insert_batch_if_new::register_lints(sess, lint_store);
    bevy_world_non_send::register_lints(sess, lint_store);
    bevy_world_non_send_mut::register_lints(sess, lint_store);
    bevy_world_resource::register_lints(sess, lint_store);
    bevy_world_resource_mut::register_lints(sess, lint_store);
    bevy_world_resource_ref::register_lints(sess, lint_store);
    // Finish with schedule-scope and zero-sized query policies.
    bevy_world_run_schedule::register_lints(sess, lint_store);
    bevy_world_schedule_scope::register_lints(sess, lint_store);
    bevy_zst_query::register_lints(sess, lint_store);
}
