#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy::app::App;
use bevy::ecs::{
    message::{Message, Messages},
    resource::Resource,
};

#[derive(Message)]
struct Ping;

#[derive(Resource, Default)]
struct Score(u32);

fn configure(app: &mut App) {
    app.init_resource::<Messages<Ping>>();
    app.insert_resource(Messages::<Ping>::default());
    app.add_message::<Ping>();
    app.init_resource::<Score>();
    app.insert_resource(Score(0));
}

fn main() {}
