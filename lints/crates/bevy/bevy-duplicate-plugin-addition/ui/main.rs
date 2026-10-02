#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_app::{App, Plugin};

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _: &mut App) {}
}

fn configure(app: &mut App) {
    app.add_plugins(GamePlugin).add_plugins(GamePlugin);
}

fn main() {}
