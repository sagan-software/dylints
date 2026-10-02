# serde-all-fields-default

## What it does

Checks for a struct that derives both `Default` and `Deserialize`, has at
least two fields, and puts a bare `#[serde(default)]` on every field.

## Why is this bad?

With a derived `Default`, a container-level `#[serde(default)]` fills each
missing field with the same value as the field-level attributes. Repeating the
attribute on every field adds noise, and a new field added without it becomes
required by mistake.

## Known problems

The lint skips a struct whose `Default` is implemented by hand, because its
values can differ from the per-field defaults. It skips a struct where any
field uses `default = "path"`.

## Example

```rust
#[derive(Default, serde::Deserialize)]
struct Settings {
    #[serde(default)]
    retries: u32,
    #[serde(default)]
    verbose: bool,
}
```

## Use instead

```rust
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Settings {
    retries: u32,
    verbose: bool,
}
```
