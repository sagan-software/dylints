# insta-empty-snapshot-suffix

## What it does

Checks for `insta::Settings::set_snapshot_suffix` calls whose argument is the
empty string literal `""`.

## Why is this bad?

Insta appends `@` and the suffix to each snapshot name. An empty suffix
renames every snapshot to end in a bare `@`, such as `name@.snap`, without
telling cases apart.

## Known problems

The lint follows string literals for at most eight steps through simple immutable
local bindings, same-crate constants, and same-crate inherent associated
constants. Trait-associated constants remain unknown because an implementation
may override a trait default. Mutable, destructured, or uninitialized bindings;
external constants, statics, and associated constants; calls; and other
computed expressions remain unknown. The lint does not check suffixes that
contain only whitespace.

## Example

```rust
use insta::Settings;

fn compact_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_suffix("");
    settings
}
```

## Use instead

Name the case, or remove the call.

```rust
use insta::Settings;

fn compact_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_suffix("compact");
    settings
}
```
