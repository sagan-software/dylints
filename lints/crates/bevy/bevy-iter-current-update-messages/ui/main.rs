#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::message::{Message, Messages};

#[derive(Message)]
struct Ping;

fn read(messages: &Messages<Ping>) {
    let _ = messages.iter_current_update_messages();
}

fn main() {}
