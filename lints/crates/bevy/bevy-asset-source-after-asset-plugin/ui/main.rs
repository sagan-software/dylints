#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    missing_docs,
    unused_crate_dependencies,
    unused_results,
    unknown_lints,
    unused_variables,
    clippy::missing_docs_in_private_items,
    reason = "This fixture keeps each lint case small and independent."
)]

use bevy_app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy_asset::{
    AssetApp, AssetPlugin,
    io::{AssetSourceBuilder, memory::MemoryAssetReader, web::WebAssetPlugin},
};

fn source_builder() -> AssetSourceBuilder {
    AssetSourceBuilder::new(|| Box::new(MemoryAssetReader::default()))
}

fn source_after_chain(app: &mut App) {
    app.add_plugins(AssetPlugin::default())
        .register_asset_source("late_chain", source_builder());
}

fn source_after_statement(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
    app.register_asset_source("late_statement", source_builder());
}

fn source_before_plugin(app: &mut App) {
    app.register_asset_source("early", source_builder());
    app.add_plugins(AssetPlugin::default());
}

fn source_in_let_else_is_checked(app: &mut App, register: bool) {
    let Some(()) = register.then_some(()) else {
        app.add_plugins(AssetPlugin::default());
        app.register_asset_source("let_else", source_builder());
        return;
    };
}

fn temporary_app_source_order_is_checked() {
    let _app = App::new()
        .add_plugins(AssetPlugin::default())
        .register_asset_source("temporary", source_builder());
}

fn web_after_asset_tuple(app: &mut App) {
    app.add_plugins((AssetPlugin::default(), WebAssetPlugin::default()));
}

fn web_before_asset_tuple(app: &mut App) {
    app.add_plugins((WebAssetPlugin::default(), AssetPlugin::default()));
}

fn web_after_asset_statement(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(WebAssetPlugin::default());
}

fn web_after_asset_tuple_binding(app: &mut App) {
    let plugins = (AssetPlugin::default(), WebAssetPlugin::default());
    app.add_plugins(plugins);
}

fn source_after_direct_app_alias(app: &mut App) {
    let alias = app;
    alias.add_plugins(AssetPlugin::default());
    alias.register_asset_source("alias", source_builder());
}

fn conditional_asset_plugin_does_not_escape(app: &mut App, enabled: bool) {
    if enabled {
        app.add_plugins(AssetPlugin::default());
    }
    app.register_asset_source("conditional", source_builder());
}

fn source_inside_branch_inherits_asset_plugin(app: &mut App, enabled: bool) {
    app.add_plugins(AssetPlugin::default());
    if enabled {
        app.register_asset_source("branch", source_builder());
    }
}

fn unrelated_assignment_preserves_asset_state(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
    let mut revision = 0;
    assert_eq!(revision, 0);
    revision = 1;
    assert_eq!(revision, 1);
    app.register_asset_source("unrelated_assignment", source_builder());
}

#[expect(
    clippy::almost_swapped,
    reason = "This fixture checks that a moved App retains its tracked identity."
)]
fn app_move_out_and_back_preserves_asset_state(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    let moved_app = app;
    app = moved_app;
    app.register_asset_source("moved_app", source_builder());
}

fn assignment_to_another_tracked_app_preserves_asset_state(mut app: App, mut source_app: App) {
    app.add_plugins(AssetPlugin::default());
    source_app.add_plugins(AssetPlugin::default());
    app = source_app;
    app.register_asset_source("other_app", source_builder());
}

struct AppHolder {
    app: App,
}

fn app_field_assignment_remains_untracked(holder: &mut AppHolder) {
    holder.app.add_plugins(AssetPlugin::default());
    holder.app = App::new();
    holder
        .app
        .register_asset_source("app_field", source_builder());
}

fn reassigned_app_binding_is_not_tracked(mut app: App, other_app: App) {
    app.add_plugins(AssetPlugin::default());
    app = other_app;
    app.register_asset_source("reassigned", source_builder());
}

fn conditional_app_reassignment_is_not_tracked(mut app: App, replace: bool) {
    app.add_plugins(AssetPlugin::default());
    if replace {
        app = App::new();
        app.add_plugins(AssetPlugin::default());
    }
    app.register_asset_source("conditional_reassignment", source_builder());
}

#[expect(
    clippy::self_assignment,
    reason = "This fixture checks that an ambiguous App identity stays unknown."
)]
fn self_assignment_after_conditional_replacement_stays_unknown(mut app: App, replace: bool) {
    app.add_plugins(AssetPlugin::default());
    if replace {
        app = App::new();
        app.add_plugins(AssetPlugin::default());
    }
    app = app;
    app.register_asset_source("ambiguous_self_assignment", source_builder());
}

fn definite_assignment_restores_asset_tracking(mut app: App, replace: bool, source_app: App) {
    app.add_plugins(AssetPlugin::default());
    if replace {
        app = App::new();
        app.add_plugins(AssetPlugin::default());
    }
    app = source_app;
    app.add_plugins(AssetPlugin::default());
    app.register_asset_source("definite_assignment", source_builder());
}

fn dereferenced_app_assignment_invalidates_asset_tracking(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
    *app = App::new();
    app.register_asset_source("dereferenced_assignment", source_builder());
}

fn loop_app_reassignment_is_not_tracked(mut app: App, replace: bool) {
    app.add_plugins(AssetPlugin::default());
    let mut remaining_iterations = usize::from(replace);
    while remaining_iterations > 0 {
        app = App::new();
        remaining_iterations -= 1;
    }
    app.register_asset_source("loop_reassignment", source_builder());
}

#[derive(Clone, Copy)]
enum AppReplacement {
    Keep,
    Replace,
}

fn match_app_reassignment_is_not_tracked(mut app: App, replacement: AppReplacement) {
    app.add_plugins(AssetPlugin::default());
    match replacement {
        AppReplacement::Replace => app = App::new(),
        AppReplacement::Keep => {}
    }
    app.register_asset_source("match_reassignment", source_builder());
}

fn plugin_addition_after_unknown_app_join_is_not_tracked(mut app: App, use_default: bool) {
    app.add_plugins(AssetPlugin::default());
    if use_default {
        app = App::default();
    } else {
        app = App::new();
    }
    app.add_plugins(AssetPlugin::default());
    app.register_asset_source("unknown_app_join", source_builder());
}

fn separate_app_bindings(asset_app: &mut App, source_app: &mut App) {
    asset_app.add_plugins(AssetPlugin::default());
    source_app.register_asset_source("separate", source_builder());
}

#[derive(Default)]
struct HiddenAssetPlugin;

impl Plugin for HiddenAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetPlugin::default());
    }
}

fn custom_plugin_body_is_unknown(app: &mut App) {
    app.add_plugins(HiddenAssetPlugin);
    app.register_asset_source("custom", source_builder());
}

fn helper_call_is_unknown(app: &mut App) {
    add_asset_plugin(app);
    app.register_asset_source("helper", source_builder());
}

#[derive(Default)]
struct DisabledAssetPluginGroup;

impl PluginGroup for DisabledAssetPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(AssetPlugin::default())
            .disable::<AssetPlugin>()
    }
}

fn disabled_asset_plugin_group_is_not_tracked(app: &mut App) {
    app.add_plugins(DisabledAssetPluginGroup);
    app.register_asset_source("disabled_group", source_builder());
}

fn add_asset_plugin(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
}

fn replace_app_through_helper(app: &mut App) {
    *app = App::new();
}

fn mutable_app_escape_invalidates_asset_facts(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
    replace_app_through_helper(app);
    app.register_asset_source("opaque_mutable_escape", source_builder());
}

struct AppMutator {
    should_replace: bool,
}

impl AppMutator {
    fn replace_app(&self, app: &mut App) {
        if self.should_replace {
            *app = App::new();
        }
    }
}

fn mutable_app_method_argument_invalidates_asset_facts(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    AppMutator {
        should_replace: true,
    }
    .replace_app(&mut app);
    app.register_asset_source("method_escape", source_builder());
}

fn invoke_opaque_action<F: FnMut()>(mut action: F) {
    action();
}

struct ActionRunner {
    should_run: bool,
}

impl ActionRunner {
    fn run<F: FnMut()>(&self, mut action: F) {
        if self.should_run {
            action();
        }
    }
}

fn called_capturing_closure_invalidates_asset_facts(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    let mut replace = || app = App::new();
    replace();
    app.register_asset_source("called_closure", source_builder());
}

fn passed_capturing_closure_invalidates_asset_facts(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    invoke_opaque_action(|| app = App::new());
    app.register_asset_source("closure_argument", source_builder());
}

fn method_closure_argument_invalidates_asset_facts(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    ActionRunner { should_run: true }.run(|| app = App::new());
    app.register_asset_source("method_closure_argument", source_builder());
}

fn uninvoked_closure_preserves_asset_facts(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    {
        let replace = || app = App::new();
    }
    app.register_asset_source("uninvoked_closure", source_builder());
}

fn escaped_moved_app_alias_invalidates_asset_facts(mut app: App) {
    app.add_plugins(AssetPlugin::default());
    let mut alias = app;
    replace_app_through_helper(&mut alias);
    alias.register_asset_source("app_alias_escape", source_builder());
}

fn unknown_app_identity_escape_invalidates_asset_facts(mut app: App, replace: bool) {
    app.add_plugins(AssetPlugin::default());
    if replace {
        app = App::new();
    }
    replace_app_through_helper(&mut app);
    app.register_asset_source("unknown_app_escape", source_builder());
}

fn nonlocal_app_reborrow_escape_invalidates_asset_facts(app: &mut App) {
    app.add_plugins(AssetPlugin::default());
    replace_app_through_helper(&mut *app);
    app.register_asset_source("nonlocal_app_escape", source_builder());
}

fn main() {}
