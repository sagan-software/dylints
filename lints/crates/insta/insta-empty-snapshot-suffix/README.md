# insta-empty-snapshot-suffix

## What it does

Checks for `insta::Settings::set_snapshot_suffix` calls whose argument is the
empty string literal `""`.

## Why is this bad?

Insta appends `@` and the suffix to each snapshot name. An empty suffix
renames every snapshot to end in a bare `@`, such as `name@.snap`, without
telling cases apart.

## Known problems

The lint checks only a string literal passed directly as the argument. It does
not check values held in constants or variables, or built at runtime. It does
not check a suffix that contains only whitespace.

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
