# insta-settings-new

## What it does

Checks for `insta::Settings::new()` calls.

## Why is this bad?

`Settings::new()` starts from Insta's defaults and drops the settings bound in
the current scope, such as redactions, filters, and the snapshot path.
Snapshots taken with the new settings can then differ from the rest of the
test suite. `Settings::clone_current()` keeps the current settings and lets
the code change only what it needs.

## Known problems

The lint warns even when a test must start from the defaults on purpose. It
does not check `Settings::default()`.

## Example

```rust
use insta::Settings;

fn compact_settings() -> Settings {
    let mut settings = Settings::new();
    settings.set_snapshot_suffix("compact");
    settings
}
```

## Use instead

```rust
use insta::Settings;

fn compact_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_suffix("compact");
    settings
}
```
