# tracing-none-record

## What it does

Checks for `record` called directly on `tracing::Span::none()`.

## Why is this bad?

`Span::none()` returns a disabled span with no fields. Recording a value on it
sends nothing to the subscriber, so the value is discarded. The call suggests
that the value reaches the trace when it does not.

## Known problems

The lint only checks a direct `Span::none().record(...)` chain. It does not
follow the span through a local binding, a field, or a helper function.

## Example

```rust
fn handle(value: u64) {
    tracing::Span::none().record("field", value);
}
```

## Use instead

Remove the call. If the value belongs in the trace, declare the field on an
enabled span and record it there:

```rust
fn handle(value: u64) {
    let span = tracing::info_span!("request", field = tracing::field::Empty);
    span.record("field", value);
}
```
