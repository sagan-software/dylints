# insta-empty-snapshot-path

## What it does

Checks for `insta::Settings::set_snapshot_path` calls whose argument is the
empty string literal `""`.

## Why is this bad?

Insta resolves a relative snapshot path from the directory of the test file.
An empty path makes Insta write snapshot files into the source directory
itself instead of the default `snapshots` directory.

## Known problems

The lint checks a string literal passed directly as the argument, or
a `const` defined in the same crate and initialized with a string literal. It does not check values held in variables or built at
runtime.

## Example

```rust
use insta::Settings;

fn api_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("");
    settings
}
```

## Use instead

Name the snapshot directory, or remove the call to keep `snapshots`.

```rust
use insta::Settings;

fn api_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("snapshots/api");
    settings
}
```
