# insta-noop-filter

## What it does

Checks for `insta::Settings::add_filter` calls whose pattern and replacement
are the same nonempty string literal with no regular expression special
characters, such as `add_filter("token", "token")`.

## Why is this bad?

The filter replaces the matched text with the same text, so it does not change
the snapshot. The test looks protected against unstable data that it still
records.

## Known problems

The lint skips patterns that contain any of `.^$*+?()[]{}|\`, even when the
filter still has no effect. It checks string literals passed directly as
arguments, and a `const` defined in the same crate and initialized with a string literal. The machine-applicable fix removes the call only
when the call is a whole statement.

## Example

```rust
use insta::Settings;

fn token_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.add_filter("token", "token");
    settings
}
```

## Use instead

Replace the match with a stable value, or remove the filter.

```rust
use insta::Settings;

fn token_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.add_filter("token", "[token]");
    settings
}
```
