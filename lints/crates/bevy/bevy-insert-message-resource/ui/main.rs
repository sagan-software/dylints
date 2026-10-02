#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_app::App;
use bevy_ecs::message::{Message, Messages};

#[derive(Message)]
struct Ping;

fn configure(app: &mut App) {
    app.init_resource::<Messages<Ping>>();
    app.add_message::<Ping>();
}

fn main() {}
