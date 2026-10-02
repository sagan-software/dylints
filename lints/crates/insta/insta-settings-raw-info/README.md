# insta-settings-raw-info

## What it does

Checks for `insta::Settings::set_raw_info` calls.

## Why is this bad?

Insta applies the redactions from `Settings::add_redaction` to metadata set
with `set_info`, but not to metadata set with `set_raw_info`. Values that the
snapshot body hides, such as tokens or timestamps, can then appear in the
stored snapshot metadata.

## Known problems

The lint warns even when the value is already safe to store. `set_info`
requires Insta's `serde` feature, so code without that feature has no
replacement.

## Example

```rust
use insta::{internals::Content, Settings};

fn user_settings(user: &str) -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_raw_info(&Content::String(user.to_owned()));
    settings
}
```

## Use instead

```rust
use insta::Settings;

fn user_settings(user: &str) -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_info(&user);
    settings
}
```
