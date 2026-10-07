# insta-no-module-prefix

## What it does

Checks for `insta::Settings::set_prepend_module_to_snapshot(false)` calls.

## Why is this bad?

By default, Insta names a snapshot file `<module>__<name>.snap`. Turning off the
prefix changes the file name to `<name>.snap`. Tests with the same name in
different modules of one directory then write to the same snapshot file.

## Known problems

The lint warns even when a project keeps snapshot names unique by other means.
It does not check a call whose argument is not the literal `false`.

## Example

```rust
use insta::Settings;

fn short_name_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings
}
```

## Use instead

Keep the default module prefix.

```rust
use insta::Settings;

fn short_name_settings() -> Settings {
    Settings::clone_current()
}
```
