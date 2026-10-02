# insta-empty-input-file

## What it does

Checks for `insta::Settings::set_input_file` calls whose argument is the empty
string literal `""`.

## Why is this bad?

Insta stores the input file path with the snapshot so reviewers can find the
input that produced it. An empty path points to no file, and it often means a
value was left out by mistake.

## Known problems

The lint checks a string literal passed directly as the argument, or
a `const` defined in the same crate and initialized with a string literal. It does not check values held in variables or built at
runtime.

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
