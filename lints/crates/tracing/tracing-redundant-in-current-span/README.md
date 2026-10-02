# tracing-redundant-in-current-span

## What it does

Checks for `in_current_span()` called directly on the result of
`Instrument::instrument(span)`.

## Why is this bad?

The call wraps the future twice, so each poll enters two spans. Tracing
documents that `span.or_current()` passed to `instrument` is more efficient:
it attaches the new span when enabled and the current span otherwise, with one
wrapper.

## Known problems

The lint reports every direct chain, whatever the span argument is. The rewrite
differs when tracing enables the inner span. In that case, the future no longer
enters the current span around the inner span on each poll. This
matters when the inner span has a different parent, such as one created with
`parent: None`.

The lint does not follow the instrumented future through a local binding or
another function.

## Example

```rust
use tracing::Instrument;

async fn work() {}

async fn run() {
    work()
        .instrument(tracing::debug_span!("work"))
        .in_current_span()
        .await;
}
```

## Use instead

```rust
use tracing::Instrument;

async fn work() {}

async fn run() {
    work()
        .instrument(tracing::debug_span!("work").or_current())
        .await;
}
```
