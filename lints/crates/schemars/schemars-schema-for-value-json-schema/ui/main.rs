//! UI fixture for value-based schemas of types implementing `JsonSchema`.
#![expect(
    unused_crate_dependencies,
    reason = "UI examples inherit the lint crate's compiler-only dependencies"
)]

use schemars::{JsonSchema, SchemaGenerator};
use serde::Serialize;

/// Schema-aware enum used to demonstrate loss of variant precision.
#[derive(JsonSchema, Serialize)]
enum Message {
    /// Text payload.
    Text(String),
    /// Numeric payload.
    Number(i64),
}

/// Serializable type that intentionally lacks `JsonSchema`.
#[derive(Serialize)]
struct SerializeOnly {
    /// Serializable payload.
    value: String,
}

/// Generate value-based schemas for a type that already implements `JsonSchema`.
fn invalid_value_schemas() {
    consume(schemars::schema_for_value!(Message::Number(7)));
    consume(SchemaGenerator::default().root_schema_for_value(&Message::Text(String::new())));
    consume(SchemaGenerator::default().into_root_schema_for_value(&Message::Text(String::new())));
}

/// Use type-based generation or a genuinely serialize-only value.
fn valid_schemas() {
    consume(schemars::schema_for!(Message));
    let value = SerializeOnly {
        value: String::new(),
    };
    consume(schemars::schema_for_value!(&value));
    assert!(value.value.is_empty());
}

/// User type with a similarly named method.
struct OtherGenerator;

impl OtherGenerator {
    /// Accept any value without generating a schema.
    const fn root_schema_for_value<T>(&self, _value: &T) -> &Self {
        self
    }
}

/// Confirm similarly named user methods do not trigger.
fn similarly_named_user_method() {
    consume(OtherGenerator.root_schema_for_value(&Message::Number(7)));
}

/// Consume a generated schema so strict fixture checks observe its result.
fn consume<T>(_value: T) {}

/// Exercise each fixture path.
fn main() {
    invalid_value_schemas();
    valid_schemas();
    similarly_named_user_method();
}
