# broad_string_error_variant

## What it does

Checks enums whose names end in `Error` or `Errors` for variants that carry a
`String` or `&str` in a field named `message`, `details`, `reason`, or
`error`, or as the only field of a tuple variant named `Message`, `Details`,
`Reason`, or `Error`.

## Why is this bad?

A free-form string turns one variant into a catch-all. Callers cannot match on
the failure kind without parsing text, the original error source is lost, and
the text can carry user data into logs.

## Known problems

The type check reads the written type name. A type alias for `String`,
`Box<str>`, or `Cow<'_, str>` is missed, and any type named `String` matches.

Enums with other names, such as `Failure`, are not checked. Other field and
variant names, such as `text` or `Description(String)`, are not checked.

## Example

```rust
enum ConfigError {
    InvalidPort { message: String },
    Reason(String),
}
```

## Use instead

```rust
enum ConfigError {
    InvalidPort { source: std::num::ParseIntError },
    MissingKey { key: &'static str },
}
```
