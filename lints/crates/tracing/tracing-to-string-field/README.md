# tracing-to-string-field

## What it does

Checks for a field in a `tracing` event or span macro whose value is
`ToString::to_string` called on a variable or field path, such as
`error = error.to_string()`.

## Why is this bad?

`to_string` allocates a `String` every time the macro runs, even before a
subscriber decides whether to record the event. The `%` sigil records the same
`Display` output without the allocation.

## Known problems

The lint checks only a receiver that is a variable or a dotted field path, so
`error().to_string()` does not trigger it. An inherent `to_string` method that
is not `ToString::to_string` does not trigger it either. In an event macro, it
checks only fields written before the message. It gives help text but no
automatic fix.

## Example

```rust
fn report(error: std::io::Error) {
    tracing::warn!(error = error.to_string(), "request failed");
}
```

## Use instead

```rust
fn report(error: std::io::Error) {
    tracing::warn!(%error, "request failed");
}
```
