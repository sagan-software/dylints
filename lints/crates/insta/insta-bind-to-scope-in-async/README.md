# insta-bind-to-scope-in-async

## What it does

Checks for `insta::Settings::bind_to_scope` calls inside an async function,
async block, or async closure. It warns when the returned guard may remain live
while the coroutine returns `Poll::Pending` at a suspension point.

## Why is this bad?

`bind_to_scope` binds the settings to the current thread until Rust drops the
returned guard. An async task can move to another thread at an `.await`, and
other tasks can run on the same thread. Snapshots in the task can then miss
the settings, and snapshots in other tasks can pick them up.
`Settings::bind_async` binds the settings to one future instead.

## Known problems

Calls inside plain closures nested in async code and calls inside synchronous
functions called by async code are not analyzed.

Mutable collection tracking is conservative. After `drop(guards.pop())`, the
lint can still warn at a later `.await` because MIR does not identify which
element the collection removed.

Opaque helper calls can also produce false positives when their return type
mentions the guard but stores no guard, such as `PhantomData<Guard>`. The lint
preserves aggregate field paths through a helper only when its MIR proves that
the helper returns its sole argument unchanged.

## Example

```rust
use insta::Settings;

async fn fetch() -> String {
    String::from("body")
}

async fn snapshot_response() {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_suffix("v2");
    let _guard = settings.bind_to_scope();
    insta::assert_snapshot!(fetch().await);
}
```

## Use instead

```rust
use insta::Settings;

async fn fetch() -> String {
    String::from("body")
}

async fn snapshot_response() {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_suffix("v2");
    settings
        .bind_async(async {
            insta::assert_snapshot!(fetch().await);
        })
        .await;
}
```
