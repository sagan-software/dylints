# module_type_first

## What it does

Checks modules that contain a struct, enum, union, or type alias whose name is
the PascalCase form of the module name, such as `UserProfile` in
`mod user_profile`. It warns when that type is not the first item after the
module's leading `use` and `extern crate` items.

## Why is this bad?

A module named after a type exists to define that type. When helpers,
constants, or impls come first, a reader must scroll past them to find the
definition that explains the rest of the module.

## Known problems

The name match is exact, so `mod api_client` does not match `APIClient`, and
the lint never treats traits as the module's type. It does not check the crate root. It ignores macro invocations and items that macros generate, both as the module's type and as items placed before it.

## Example

```rust
mod user_profile {
    use std::fmt;

    fn normalize_name(raw: &str) -> String {
        raw.trim().to_owned()
    }

    struct UserProfile {
        name: String,
    }
}
```

## Use instead

```rust
mod user_profile {
    use std::fmt;

    struct UserProfile {
        name: String,
    }

    fn normalize_name(raw: &str) -> String {
        raw.trim().to_owned()
    }
}
```
