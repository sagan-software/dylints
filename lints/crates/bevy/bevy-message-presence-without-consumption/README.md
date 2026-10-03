# bevy-message-presence-without-consumption

## What it does

Checks for wildcard bindings that discard direct calls to
`MessageReader::read()` or `MessageReader::read_with_id()`.

## Why is this bad?

Both methods create lazy iterators. The reader advances its cursor as an
iterator is consumed, so dropping it leaves unread messages available to the
next read.

## Known problems

The lint reports only wildcard `let` initializers that are direct,
method-syntax calls to `read()` or `read_with_id()`. Bevy's iterator types do
not carry a `must_use` attribute, so rustc and default Clippy do not diagnose
these direct wildcard bindings. Clippy's opt-in `let_underscore_must_use`
lint diagnoses standard iterator adapter chains, so this lint does not inspect
those chains.

Strict configurations that enable `unused_results` already
diagnose semicolon expressions. It offers no automatic fix because processing
messages and intentionally discarding them require different code. Repeated
`is_empty()` checks can be intentional. The lint does not analyze helper
functions, local variables, explicit `drop`, or control flow.

## Example

```rust
use bevy_ecs::message::{Message, MessageReader};

#[derive(Message)]
struct Collision;

fn play_collision_sound() {}

fn play_sound(mut collisions: MessageReader<Collision>) {
    let _ = collisions.read();
    play_collision_sound();
}
```

## Use instead

```rust
use bevy_ecs::message::{Message, MessageReader};

#[derive(Message)]
struct Collision;

fn play_collision_sound() {}

fn play_sound(mut collisions: MessageReader<Collision>) {
    for _collision in collisions.read() {
        play_collision_sound();
    }
}
```
