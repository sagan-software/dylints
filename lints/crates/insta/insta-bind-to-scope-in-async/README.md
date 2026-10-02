# insta-bind-to-scope-in-async

## What it does

Checks for `insta::Settings::bind_to_scope` calls inside an async function,
async block, or async closure.

## Why is this bad?

`bind_to_scope` binds the settings to the current thread until the returned
guard is dropped. An async task can move to another thread at an `.await`, and
other tasks can run on the same thread. Snapshots in the task can then miss
the settings, and snapshots in other tasks can pick them up.
`Settings::bind_async` binds the settings to one future instead.

## Known problems

The lint warns even when no `.await` occurs while the guard is alive. It does
not check a call inside a plain closure within async code, or inside a
synchronous helper function called from async code.

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
