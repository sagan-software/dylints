# tracing-format-field

## What it does

Checks for a field in a `tracing` event or span macro whose value is a
`format!` call, such as `request = format!("{request_id}:{status}")`.

## Why is this bad?

`format!` allocates a `String` every time the macro runs, even before a
subscriber decides whether to record the event. It also joins several values
into one text field, so a subscriber cannot filter or group by each value
without parsing the text. Tracing can record each value as its own field and
format it with the `%` (`Display`) or `?` (`Debug`) sigil only when needed.

## Known problems

Some fields must hold a composed string, for example to match an external log
schema. The lint warns on those fields too.

The lint checks only a field value that is exactly `format!(...)`,
`std::format!(...)`, or `::std::format!(...)`. In an event macro, it checks
only fields written before the message. It gives help text but no automatic
fix.

## Example

```rust
fn handle(request_id: u64, status: &str) {
    tracing::info!(request = format!("{request_id}:{status}"));
}
```

## Use instead

```rust
fn handle(request_id: u64, status: &str) {
    tracing::info!(request_id, %status);
}
```
