# tracing-redundant-in-current-span

## What it does

Checks for `in_current_span()` called directly on the result of
`Instrument::instrument(span)`.

## Why is this bad?

The future is wrapped twice and enters two spans on every poll. Tracing
documents that `span.or_current()` passed to `instrument` is more efficient:
it attaches the new span when it is enabled and the current span otherwise,
with one wrapper.

## Known problems

The lint reports every direct chain, whatever the span argument is. The
rewrite is not equivalent when the inner span is enabled: the future then no
longer enters the current span around the inner span on each poll. This
matters when the inner span has a different parent, such as one created with
`parent: None`.

The lint does not follow the instrumented future through a local binding or a
helper function.

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
