# tracing-message-interpolation

## What it does

Checks for a variable or field path that appears in the message of a `tracing`
event macro, such as `info!` or `event!`, but not in a structured field of the
same event. It covers captured arguments such as `{request_id}` and positional
arguments such as `"request {} failed", request_id`.

## Why is this bad?

A value that appears only in the message is stored as part of one text field.
Subscribers cannot filter, group, or export it as a named field without parsing
the message, and that parsing breaks when the wording changes.

## Known problems

The lint checks only variables and dotted field paths. A function call or
other expression in the message does not trigger it. It does not check span
macros such as `info_span!`.

It gives help text but no automatic fix.

## Example

```rust
fn complete(request_id: u64) {
    tracing::info!("request {request_id} completed");
}
```

## Use instead

Record the value as a field. A message may still repeat a value that is also
recorded as a field.

```rust
fn complete(request_id: u64) {
    tracing::info!(request_id, "request completed");
}
```
