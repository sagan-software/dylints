# insta-empty-filter-pattern

## What it does

Checks for `insta::Settings::add_filter` calls whose pattern is the empty
string literal `""`.

## Why is this bad?

Insta applies each filter as a regular expression replacement on the snapshot
text. An empty pattern matches at every position, so the replacement inserts
text between every character and the snapshot no longer shows the value under
test.

## Known problems

The lint follows string literals for at most eight steps through simple immutable
local bindings, same-crate constants, and same-crate inherent associated
constants. Trait-associated constants remain unknown, even with literal defaults,
because an implementation may override the default.
Mutable, destructured, or uninitialized bindings;
external constants, statics, and associated constants; calls; and other
computed expressions remain unknown. The lint does not check other patterns
that can match empty text, such as `"a*"`.

## Example

```rust
use insta::Settings;

fn id_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.add_filter("", "[id]");
    settings
}
```

## Use instead

```rust
use insta::Settings;

fn id_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.add_filter(r"\b[[:xdigit:]]{32}\b", "[id]");
    settings
}
```
