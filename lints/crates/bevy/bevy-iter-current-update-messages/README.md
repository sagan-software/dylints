# bevy-iter-current-update-messages

## What it does

Checks for calls to `Messages::iter_current_update_messages`.

## Why is this bad?

The method only returns messages written since the last `Messages::update` call. The next update drops messages that code writes after the call and before the update, without reading them.

## Known problems

The lint reports every call, including code that runs at a point where that window is correct.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Message)] struct Ping;
fn count_pings(messages: Res<Messages<Ping>>) -> usize {
    messages.iter_current_update_messages().count()
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Message)] struct Ping;
fn count_pings(mut pings: MessageReader<Ping>) -> usize {
    pings.read().count()
}
```
