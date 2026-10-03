#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_variables
)]

use bevy_ecs::message::{Message, MessageReader as BevyReader};

#[derive(Message)]
struct Ping;

fn ignored_reader(mut reader: BevyReader<Ping>) {
    let _ = reader.read();
}

fn count_messages(mut reader: BevyReader<Ping>) {
    let count = reader.read().count();
    let _ = count;
}

fn visit_messages(mut reader: BevyReader<Ping>) {
    reader.read().for_each(|_message| {});
}

fn loop_over_messages(mut reader: BevyReader<Ping>) {
    for _message in reader.read() {}
}

fn clear_messages(mut reader: BevyReader<Ping>) {
    reader.clear();
}

fn consume_one_message(mut reader: BevyReader<Ping>) {
    let first = reader.read().next();
    let _ = first;
}

fn ignored_reader_with_id(mut reader: BevyReader<Ping>) {
    let _ = reader.read_with_id();
}

fn count_messages_with_id(mut reader: BevyReader<Ping>) {
    let count = reader.read_with_id().count();
    let _ = count;
}

fn discard_adapter_chain(mut reader: BevyReader<Ping>) {
    let _ = reader.read().map(|message| message);
}

fn explicitly_drop_iterator(mut reader: BevyReader<Ping>) {
    drop(reader.read());
}

fn discard_read_expression(mut reader: BevyReader<Ping>) {
    reader.read();
}

fn discard_read_with_id_expression(mut reader: BevyReader<Ping>) {
    reader.read_with_id();
}

struct MessageReader;

impl MessageReader {
    fn read(&mut self) -> core::iter::Empty<()> {
        core::iter::empty()
    }
}

fn unrelated_reader(mut reader: MessageReader) {
    let _ = reader.read();
}

fn uninitialized_reader_wildcard_does_not_trigger() {
    let _: BevyReader<Ping>;
}

fn main() {}
