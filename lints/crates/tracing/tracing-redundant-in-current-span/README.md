# tracing-redundant-in-current-span

## What it does

Reports direct chains of `Instrument::instrument(span)` followed by
`in_current_span()`, with advice to use `span.or_current()` when the inner span
is disabled.

## Why is this bad?

`Instrument::instrument` and `Instrument::in_current_span` each attach a span
that is entered whenever the instrumented future is polled or dropped. See the
[tracing 0.1.44 Instrument documentation](https://docs.rs/tracing/0.1.44/tracing/instrument/trait.Instrument.html).
`Span::or_current` selects the current span only when the supplied span is
disabled. See the [tracing 0.1.44 `Span::or_current` documentation](https://docs.rs/tracing/0.1.44/tracing/struct.Span.html#method.or_current).

When `outer` is current and an enabled parentless `inner` span is used, each
poll and drop of the original chain calls `on_enter(outer)`, `on_enter(inner)`,
`on_exit(inner)`, then `on_exit(outer)`. When those same spans are used with
`instrument(inner.or_current())`, each poll and drop calls only
`on_enter(inner)` and `on_exit(inner)`. The recording subscriber test asserts
both callback sequences.

## Known problems

The lint cannot determine whether the runtime subscriber enables the inner
span, so it reports a direct chain even when that span is enabled. If `outer`
is current and an enabled parentless `inner` span is used, replacing the chain
with `instrument(inner.or_current())` removes `on_enter(outer)` and
`on_exit(outer)` on each poll and drop.

The lint checks the direct method chain. It does not follow an instrumented
future through a local binding or another function.

## Example

```rust
use tracing::{Instrument, info_span};

async fn work() {}

fn drop_inner() {
    let _outer = info_span!("outer").entered();
    let _future = work()
        .instrument(info_span!(parent: None, "inner"))
        .in_current_span();
}
```

## Use instead

If the subscriber disables the inner span, pass it through `or_current()`
before `instrument`:

```rust
use tracing::{Instrument, info_span};

async fn work() {}

fn drop_inner() {
    let _outer = info_span!("outer").entered();
    let inner = info_span!(parent: None, "inner");
    let _future = work().instrument(inner.or_current());
}
```

Otherwise, keep the original chain when the captured current span's enter and
exit callbacks are required. Accept their removal only when that change is
intended.
