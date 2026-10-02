# schemars-json-schema-ref-return

## What it does

Checks for a manual `JsonSchema::json_schema` implementation that returns a
schema built with `Schema::new_ref`.

## Why is this bad?

Schemars documents that `json_schema` should not return a `$ref` schema. The
generator decides when to emit a `$ref` and registers the target definition. A
hand-written `$ref` can point to a definition that the generator never adds, so
the output schema has a dangling reference.

## Known problems

The lint only finds `Schema::new_ref` as the tail expression, as a `return`
value, or as the result of an `if` or `match` branch. It misses a `$ref` stored
in a variable before it is returned, a `$ref` converted with `.into()`, and a
`$ref` written with `json_schema!`.

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
