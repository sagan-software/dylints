# tracing-direct-record-all

## What it does

Checks for a direct call to the `tracing::Span::record_all` method in source
code that no macro produces.

## Why is this bad?

Tracing 0.1.42 removed `Span::record_all` from the documented API and
recommends calls through tracing macros. Its `ValueSet` argument has no
documented constructor, so a direct call depends on tracing internals that can
change in any release. The `tracing::record_all!` macro records several fields
through the supported API.

## Known problems

The lint skips every call produced by a macro expansion, including calls in
your own `macro_rules!` macros. It does not follow calls made through another
function.

## Example

```rust
fn record(span: &tracing::Span) {
    if let Some(metadata) = span.metadata() {
        span.record_all(&tracing::valueset!(metadata.fields(), answer = 42));
    }
}
```

## Use instead

```rust
fn record(span: &tracing::Span) {
    tracing::record_all!(span, answer = 42);
}
```
