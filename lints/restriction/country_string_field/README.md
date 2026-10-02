# country_string_field

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

The lint checks only the exact types `String` and `&str`, after resolving type
aliases and `use` renames. It does not flag `Option<String>`, `Vec<String>`,
`Box<str>`, or `Cow<'_, str>`.

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
