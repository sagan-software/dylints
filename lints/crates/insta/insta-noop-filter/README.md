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

The lint follows string literals for at most eight steps through simple immutable
local bindings, same-crate constants, and same-crate inherent associated
constants. Trait-associated constants remain unknown, even with literal defaults,
because an implementation may override the default.
Mutable, destructured, or uninitialized bindings;
external constants, statics, and associated constants; calls; and other
computed expressions remain unknown. It skips patterns that contain any of
`.^$*+?()[]{}|\\`, even when the filter still has no effect. The
machine-applicable fix removes the call only when it is a whole statement. The
fix leaves local declarations and other statements unchanged.

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
