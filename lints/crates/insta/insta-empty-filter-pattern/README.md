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

The lint follows a string literal through at most eight simple immutable local
bindings or constants defined in the same crate. Mutable, destructured, or
uninitialized bindings, external constants and statics, calls, and other
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
