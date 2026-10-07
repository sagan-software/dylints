# insta-settings-new

## What it does

Checks for `insta::Settings::new()`, `Settings::default()`, and
`Default::default()` calls that create `insta::Settings`.

## Why is this bad?

`Settings::new()` starts from Insta's defaults and drops the settings bound in
the current scope, such as redactions, filters, and the snapshot path.
Snapshots taken with the new settings can then differ from the rest of the
test suite. `Settings::clone_current()` keeps the current settings and lets
the code change only what it needs.

## Known problems

The lint warns even when a test must start from the defaults on purpose. The
machine-applicable fix renames the constructor only when the call names the
type, as in `Settings::new()`. A trait path such as `Default::default()` gets
help text only.

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
