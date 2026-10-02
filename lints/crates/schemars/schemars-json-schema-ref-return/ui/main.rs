//! UI fixture for direct `$ref` returns from `JsonSchema::json_schema`.
#![expect(
    unused_crate_dependencies,
    reason = "UI examples inherit the lint crate's compiler-only dependencies"
)]

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

/// Type whose schema method returns a reference as its tail expression.
struct DirectRef;

impl JsonSchema for DirectRef {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DirectRef".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        Schema::new_ref("#/$defs/Other".to_owned())
    }
}

/// Type whose schema method uses an explicit reference-schema return.
struct ReturnedRef;

impl JsonSchema for ReturnedRef {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ReturnedRef".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        if generator.definitions().is_empty() {
            return Schema::new_ref("#/$defs/Other".to_owned());
        }

        true.into()
    }
}

/// Type whose schema method returns a concrete schema.
struct ConcreteSchema;

impl JsonSchema for ConcreteSchema {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ConcreteSchema".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string" })
    }
}

/// Type that uses a reference schema as a nested value.
struct NestedRef;

impl JsonSchema for NestedRef {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "NestedRef".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        let nested = Schema::new_ref("#/$defs/Other".to_owned());
        json_schema!({ "anyOf": [nested] })
    }
}

/// Exercise each fixture type so strict example-target lints stay meaningful.
fn main() {
    let _ = std::hint::black_box((DirectRef, ReturnedRef, ConcreteSchema, NestedRef));
}
