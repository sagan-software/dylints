# insta-allow-empty-glob

## What it does

Checks for `insta::Settings::set_allow_empty_glob(true)` calls.

## Why is this bad?

By default, `insta::glob!` fails the test when its pattern matches no files.
That catches a misspelled pattern or a moved fixture directory. With the
setting enabled, a glob that matches nothing runs no assertions and the test
still passes.

## Known problems

The lint warns even when an empty fixture set is valid, such as fixtures that
exist only on some platforms. It does not check a call whose argument is not
the literal `true`.

## Example

```rust
use insta::Settings;

fn fixture_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_allow_empty_glob(true);
    settings
}
```

## Use instead

Keep the default, so an empty glob fails the test.

```rust
use insta::Settings;

fn fixture_settings() -> Settings {
    Settings::clone_current()
}
```
