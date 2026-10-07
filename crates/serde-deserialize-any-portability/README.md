# serde-deserialize-any-portability

## What it does

Checks for calls to `serde::Deserializer::deserialize_any`, written either as
a method call or as a path call. The lint skips calls inside an implementation
of `Deserializer`, because a format forwards to its own `deserialize_any` by
design.

## Why is this bad?

`deserialize_any` asks the input to say what type comes next. Only
self-describing formats such as JSON can do that. Formats such as Postcard and
Bincode return an error, so they cannot decode the type.

## Known problems

Some types need dynamic input, such as a type that accepts either a string or
a number. These calls also trigger the lint.

## Example

```rust
use serde::de::{Deserializer, Visitor};

struct IdVisitor;

impl<'de> Visitor<'de> for IdVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an id")
    }

    fn visit_u64<E>(self, value: u64) -> Result<u64, E> {
        Ok(value)
    }
}

fn deserialize_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    deserializer.deserialize_any(IdVisitor)
}
```

## Use instead

Call the `deserialize_*` method for the type the visitor expects:

```rust
# use serde::de::Visitor;
# struct IdVisitor;
# impl<'de> Visitor<'de> for IdVisitor {
#     type Value = u64;
#     fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { formatter.write_str("an id") }
#     fn visit_u64<E>(self, value: u64) -> Result<u64, E> { Ok(value) }
# }
use serde::de::Deserializer;

fn deserialize_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    deserializer.deserialize_u64(IdVisitor)
}
```
