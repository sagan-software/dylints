# tracing-await-holding-span-guard

## What it does

Checks for a `tracing::span::Entered` or `tracing::span::EnteredSpan` guard
that a resolved `Span::enter` or `Span::entered` call returns and that an async
future retains in its saved state at an `.await`. The lint follows owned moves
through `Option`, `Box::new`, tuples, and user-defined struct fields.

## Why is this bad?

While the future waits at the `.await`, the span stays entered. Other tasks then
run on the same thread inside that span, and tracing records their events under
it. Tracing documents this pattern as producing incorrect traces.

## Known problems

The analysis joins branch states as may-live facts. It can report a guard when
it cannot prove that every path released the guard before suspension.

When `Option::take` uses an alias that can refer to several wrappers, the
analysis transfers a possible result and keeps each source as may-live. It can
warn when branch conditions would prove that the guard was removed. An
unresolved receiver alias also leaves the source as may-live; a single known
source is cleared.

The analysis follows at most sixteen aggregate fields and sixteen recursive
wrapper types. A guard at either limit can trigger the lint. Deeper wrappers
stay quiet because the analysis stops at those bounds.

The type walk stops when it revisits an active recursive type, then checks
sibling fields.

For an unrecognized helper that consumes a guard by value, the analysis clears
the input and does not infer ownership from the helper's return value. It can
miss a guard that such a helper returns inside a wrapper.

For an unrecognized helper that mutably borrows a tracked wrapper, the analysis
marks that wrapper's ownership as unknown and stops tracking it. A helper that
leaves the guard in place can therefore hide a guard from the lint.
If the analysis cannot resolve the mutable alias, it keeps the prior ownership
state and can warn after the helper releases the guard.

The analysis does not follow guards through arrays, slices, references, raw
pointers, trait objects, closures, coroutine or future captures, generic
parameters, or unresolved associated-type projections. It recognizes
`Box::new` and resolved `Option::take`; opaque container helpers remain
unsupported.

## Example

```rust
struct Request {
    guard: Option<tracing::span::EnteredSpan>,
}

async fn other_work() {}

async fn work(span: tracing::Span) {
    let _request = Request {
        guard: Some(span.entered()),
    };
    other_work().await;
}
```

## Use instead

Use `Span::in_scope` for synchronous work, or instrument the future.

```rust
use tracing::Instrument;

async fn other_work() {}

async fn work(span: &tracing::Span) {
    other_work().instrument(span.clone()).await;
}
```
