# bevy-main-return-without-app-exit

## What it does

Checks for a free function named `main` that returns `()` and calls `App::run` as a statement,
discarding the returned `AppExit`.

## Why is this bad?

`App::run` returns the `AppExit` that the app requested. When `main` discards it, the process exits
with status 0 even after `AppExit::error()`, so scripts and CI cannot detect the failure.

## Known problems

The lint checks every free function named `main`, including one inside a module. It does not
report `let _ = app.run();`.

## Example

```rust
# use bevy::prelude::*;
fn main() {
    App::new().run();
}
```

## Use instead

```rust
# use bevy::prelude::*;
fn main() -> AppExit {
    App::new().run()
}
```
