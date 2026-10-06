# schemars-json-schema-ref-return

## What it does

Checks for a manual `JsonSchema::json_schema` implementation that returns a
schema built with `Schema::new_ref`.

## Why is this bad?

Schemars advises implementations against returning a `$ref` schema from
`json_schema`. The generator decides when to emit a `$ref` and registers the
target definition. A hand-written `$ref` can point to a definition that the
generator never adds, so the output schema has a dangling reference.

## Known problems

The lint only finds `Schema::new_ref` as the tail expression, as a `return`
value, or as the result of an `if` or `match` branch. The code can store a
`$ref` in a variable before returning it, convert a `$ref` with `.into()`, or
write a `$ref` with `json_schema!` without triggering it.

## Example

```rust
use schemars::{JsonSchema, Schema, SchemaGenerator};

struct Wrapper;

impl JsonSchema for Wrapper {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Wrapper".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        Schema::new_ref("#/$defs/Other".to_owned())
    }
}
```

## Use instead

Let the generator create the reference with `subschema_for`:

```rust
# #[derive(schemars::JsonSchema)] struct Other { value: String }
use schemars::{JsonSchema, Schema, SchemaGenerator};

struct Wrapper;

impl JsonSchema for Wrapper {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Wrapper".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<Other>()
    }
}
```
