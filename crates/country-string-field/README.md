# country-string-field

## What it does

Checks for named fields whose type is `String` or `&str` and whose name marks a
country code. The names are `country`, `country_code`, `iso_country`,
`iso_country_code`, `residence_country`, `nationality_country`, and any name
ending in `_country` or `_country_code`, compared without case.

## Why is this bad?

A string field accepts any text, so invalid or inconsistently cased codes such
as `"usa"` or `"Us"` move through the program unchecked. A country-code type
validates the value once, at the boundary.

## Known problems

The compiler resolves type aliases before the lint examines the type. The lint
peels up to eight consecutive standard `Option` layers at each point in its
traversal. Longer chains, local `Option` lookalikes, and user-defined wrappers
remain opaque. After peeling, it checks only `String` and
`&str`, including type aliases and `use` renames. It does not inspect
`Vec<String>`, `Box<str>`, or `Cow<'_, str>`.

It can warn on a field such as `home_country` that holds a display name instead
of a code. It does not flag other names, such as `country_name` or `nation`.

## Example

```rust
struct Profile<'a> {
    country: String,
    residence_country: &'a str,
}
```

## Use instead

```rust
struct CountryCode(String);

struct Profile {
    country: CountryCode,
    residence_country: CountryCode,
}
```
