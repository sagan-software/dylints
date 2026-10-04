# insta-empty-input-file

## What it does

Checks for `insta::Settings::set_input_file` calls whose argument is the empty
string literal `""`.

## Why is this bad?

Insta stores the input file path with the snapshot so reviewers can find the
input that produced it. An empty path points to no file, and it often means the
author left out a value by mistake.

## Known problems

The lint follows string literals for at most eight steps through simple immutable
local bindings, same-crate constants, and same-crate inherent associated
constants. Trait-associated constants remain unknown, even with literal defaults,
because an implementation may override the default.
Mutable, destructured, or uninitialized bindings;
external constants, statics, and associated constants; calls; and other
computed expressions remain unknown.

## Example

```rust
use insta::Settings;

fn user_fixture_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_input_file("");
    settings
}
```

## Use instead

Name the input file, or remove the call.

```rust
use insta::Settings;

fn user_fixture_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_input_file("fixtures/user.json");
    settings
}
```
