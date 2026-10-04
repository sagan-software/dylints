# insta-empty-description

## What it does

Checks for `insta::Settings::set_description` calls whose argument is the empty
string literal `""`.

## Why is this bad?

Insta shows the description next to the snapshot during review. An empty
description shows nothing, and it often means the author left out a value by
mistake.

## Known problems

The lint follows a string literal through at most eight simple immutable local
bindings or constants defined in the same crate. Mutable, destructured, or
uninitialized bindings, external constants and statics, calls, and other
computed expressions remain unknown. The lint does not check descriptions that
contain only whitespace.

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
