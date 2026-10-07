# schemars-schema-for-value-json-schema

## What it does

Checks for `schema_for_value!`, `SchemaGenerator::root_schema_for_value`, and
`SchemaGenerator::into_root_schema_for_value` called with a value whose type
implements `JsonSchema` in the current crate.

## Why is this bad?

A schema built from a value only describes that one value. For an enum, it
covers only the variant the value holds, so the schema rejects valid data that
uses another variant. Schemars documents that type-based generation gives a
more precise schema.

## Known problems

The lint only checks values whose type is a struct, enum, or union defined in
the current crate, with or without references. It misses values of external
types, primitives, and wrappers such as `Vec<T>`, even when they implement
`JsonSchema`.

## Example

```rust
#[derive(schemars::JsonSchema, serde::Serialize)]
enum Message {
    Text(String),
    Number(i64),
}

fn message_schema() -> schemars::Schema {
    schemars::schema_for_value!(Message::Number(7))
}
```

## Use instead

```rust
#[derive(schemars::JsonSchema, serde::Serialize)]
enum Message {
    Text(String),
    Number(i64),
}

fn message_schema() -> schemars::Schema {
    schemars::schema_for!(Message)
}
```
