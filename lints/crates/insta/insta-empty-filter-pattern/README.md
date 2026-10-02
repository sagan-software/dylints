# insta-empty-filter-pattern

## What it does

Checks for `insta::Settings::add_filter` calls whose pattern is the empty
string literal `""`.

## Why is this bad?

Insta applies each filter as a regular expression replacement on the snapshot
text. An empty pattern matches at every position, so the replacement is
inserted between every character and the snapshot no longer shows the value
under test.

## Known problems

The lint checks a string literal passed directly as the pattern, or
a `const` defined in the same crate and initialized with a string literal. It does not check patterns held in variables or built at runtime. It
does not check other patterns that can match empty text, such as `"a*"`.

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
