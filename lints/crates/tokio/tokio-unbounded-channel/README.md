# tokio-unbounded-channel

## What it does

Checks for calls to `tokio::sync::mpsc::unbounded_channel`.

## Why is this bad?

The sender of an unbounded channel never waits for capacity. When producers
send faster than the consumer receives, queued messages grow until the process
runs out of memory. A bounded channel makes producers wait instead.

## Known problems

Some channels carry a message count that is bounded by other code, or need a
sender that can send from synchronous code without waiting. The lint cannot
see those bounds and warns on every call.

## Example

```rust
fn make_queue() {
    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
}
```

## Use instead

Use a bounded channel sized for the expected burst, and handle the waiting
this adds to producers.

```rust
fn make_queue() {
    let (_tx, _rx) = tokio::sync::mpsc::channel::<u8>(64);
}
```
