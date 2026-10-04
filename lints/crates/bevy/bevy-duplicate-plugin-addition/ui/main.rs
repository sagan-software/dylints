// run-rustfix
// rustfix-only-machine-applicable
//! Compile-only controls for the duplicate plugin addition lint.

#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    unknown_lints,
    unused_results,
    reason = "The fixture contains standalone examples that the UI test does not execute."
)]
#![allow(
    unused_crate_dependencies,
    reason = "Cargo passes the Dylint package dependencies to this example target."
)]

use bevy_app::{App, Plugin, PluginGroup, PluginGroupBuilder, ScheduleRunnerPlugin};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts calls to the side-effecting plugin constructor.
static PLUGIN_CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

/// Local unique plugin used by duplicate controls.
struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _: &mut App) {}
}

/// Different local unique plugin used by history controls.
struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, _: &mut App) {}
}

/// Local plugin that explicitly allows repeated additions.
struct RepeatablePlugin;

impl Plugin for RepeatablePlugin {
    fn build(&self, _: &mut App) {}

    fn is_unique(&self) -> bool {
        false
    }
}

/// Local plugin group used to test the unsupported group path.
struct GamePluginGroup;

impl PluginGroup for GamePluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(GamePlugin)
    }
}

/// Return a plugin after recording its constructor side effect.
fn game_plugin() -> GamePlugin {
    PLUGIN_CONSTRUCTIONS.fetch_add(1, Ordering::Relaxed);
    GamePlugin
}

/// Preserve chain warning, non-warning, and rustfix controls.
fn configure(app: &mut App) {
    app.add_plugins(GamePlugin).add_plugins(GamePlugin);
    app.add_plugins(game_plugin()).add_plugins(game_plugin());
    app.add_plugins(GamePlugin).add_plugins(AudioPlugin);
    app.add_plugins(RepeatablePlugin)
        .add_plugins(RepeatablePlugin);
    app.add_plugins(GamePlugin);
}

/// Keep generic plugin aliases outside direct local-type analysis.
fn configure_generic<P: Plugin + Clone>(app: &mut App, plugin: P) {
    app.add_plugins(plugin.clone()).add_plugins(plugin);
}

/// Check duplicate and distinct local plugin types in tuples.
fn configure_tuple(app: &mut App) {
    app.add_plugins((GamePlugin, GamePlugin));
    app.add_plugins((GamePlugin, AudioPlugin));
    app.add_plugins((RepeatablePlugin, RepeatablePlugin));
}

/// Clear one App history after a tuple duplicate while preserving another.
fn configure_tuple_invalidation(app: &mut App, other: &mut App) {
    app.add_plugins(GamePlugin);
    other.add_plugins(AudioPlugin);
    app.add_plugins((GamePlugin, GamePlugin));
    other.add_plugins(AudioPlugin);
}

/// Check repeated direct additions to the same App.
fn configure_separate(app: &mut App) {
    app.add_plugins(GamePlugin);
    app.add_plugins(GamePlugin);
}

/// Record a distinct plugin type before checking its later duplicate.
fn configure_distinct_separate(app: &mut App) {
    app.add_plugins(GamePlugin);
    app.add_plugins(AudioPlugin);
    app.add_plugins(AudioPlugin);
}

/// Preserve both constructor evaluations for duplicate additions.
fn configure_separate_side_effects(app: &mut App) {
    app.add_plugins(game_plugin());
    app.add_plugins(game_plugin());
}

/// Keep identical plugin types on separate App values quiet.
fn configure_separate_apps() {
    let mut first = App::new();
    let mut second = App::new();
    first.add_plugins(GamePlugin);
    second.add_plugins(GamePlugin);
}

/// Keep plugins with an explicit uniqueness override quiet.
fn configure_repeatable_separate(app: &mut App) {
    app.add_plugins(RepeatablePlugin);
    app.add_plugins(RepeatablePlugin);
}

/// Keep external plugin implementations outside local-type analysis.
fn configure_external_plugin(app: &mut App) {
    app.add_plugins(ScheduleRunnerPlugin::run_once());
    app.add_plugins(ScheduleRunnerPlugin::run_once());
}

/// Keep external plugin types in tuples outside local-type analysis.
fn configure_external_plugin_tuple(app: &mut App) {
    app.add_plugins((
        ScheduleRunnerPlugin::run_once(),
        ScheduleRunnerPlugin::run_once(),
    ));
}

/// Keep function-item plugin implementations outside local-type analysis.
fn add_plugins_with_function(app: &mut App) {
    app.add_plugins(noop_plugin);
    app.add_plugins(noop_plugin);
    app.add_plugins((noop_plugin, noop_plugin));
}

/// Empty function-item plugin used by the Bevy blanket implementation.
const fn noop_plugin(_: &mut App) {}

/// Mutate an App through a helper to test provenance invalidation.
fn add_plugins_through_helper(app: &mut App) {
    app.add_plugins(AudioPlugin);
}

/// Forget App history after an unknown helper may add a plugin.
fn configure_unknown_mutation(app: &mut App) {
    app.add_plugins(GamePlugin);
    add_plugins_through_helper(app);
    app.add_plugins(GamePlugin);
}

/// Return a mutable App alias without preserving its identity.
const fn passthrough_app(app: &mut App) -> &mut App {
    app
}

/// Forget App history after an opaque alias may add a plugin.
fn configure_opaque_alias(app: &mut App) {
    app.add_plugins(GamePlugin);
    let alias = passthrough_app(app);
    alias.add_plugins(AudioPlugin);
    app.add_plugins(GamePlugin);
}

/// Reset provenance after the tracked binding changes App values.
fn configure_reassignment<'a>(mut app: &'a mut App, replacement: &'a mut App) {
    app.add_plugins(GamePlugin);
    app = replacement;
    app.add_plugins(GamePlugin);
}

/// Keep `PluginGroup` values outside direct Plugin type analysis.
fn configure_plugin_group(app: &mut App) {
    app.add_plugins(GamePluginGroup);
    app.add_plugins(GamePluginGroup);
}

/// Reset history after an unrelated App method call.
fn configure_unknown_app_method(app: &mut App) {
    app.add_plugins(GamePlugin);
    app.finish();
    app.add_plugins(GamePlugin);
}

/// Reset history when a closure captures and mutates the App.
fn configure_closure_mutation(app: &mut App) {
    app.add_plugins(GamePlugin);
    let mut add_audio = || {
        app.add_plugins(AudioPlugin);
    };
    add_audio();
    app.add_plugins(GamePlugin);
}

fn main() {}
