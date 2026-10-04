# insta-empty-description

## What it does

Checks for `insta::Settings::set_description` calls whose argument is the empty
string literal `""`.

## Why is this bad?

Insta shows the description next to the snapshot during review. An empty
description shows nothing, and it often means the author left out a value by
mistake.

## Known problems

The lint follows string literals for at most eight steps through simple immutable
local bindings, same-crate constants, and same-crate inherent associated
constants. Trait-associated constants remain unknown because an implementation
may override a trait default. Mutable, destructured, or uninitialized bindings;
external constants, statics, and associated constants; calls; and other
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
