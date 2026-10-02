# insta-empty-description

## What it does

Checks for `insta::Settings::set_description` calls whose argument is the empty
string literal `""`.

## Why is this bad?

Insta shows the description next to the snapshot during review. An empty
description shows nothing, and it often means a value was left out by mistake.

## Known problems

The lint checks a string literal passed directly as the argument, or
a `const` defined in the same crate and initialized with a string literal. It does not check values held in variables or built at
runtime. It does
not check a description that contains only whitespace.

## Example

```rust
use insta::Settings;

fn expired_token_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_description("");
    settings
}
```

## Use instead

Describe the case, or remove the call.

```rust
use insta::Settings;

fn expired_token_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_description("response when the token has expired");
    settings
}
```
