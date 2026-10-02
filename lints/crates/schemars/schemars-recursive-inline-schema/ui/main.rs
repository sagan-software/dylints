//! UI fixture for recursive schemas configured to inline.
#![expect(
    unused_crate_dependencies,
    reason = "UI examples inherit the lint crate's compiler-only dependencies"
)]

use schemars::{JsonSchema, Schema, SchemaGenerator};

/// Derived recursive type incorrectly configured to inline.
#[derive(JsonSchema)]
#[schemars(inline)]
struct InlineNode {
    /// Optional recursive link.
    next: Option<Box<Self>>,
}

/// Manually implemented recursive type that reports itself as inline.
struct ManualInlineNode {
    /// Recursive children.
    children: Vec<Self>,
}

impl JsonSchema for ManualInlineNode {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ManualInlineNode".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        true.into()
    }
}

/// Recursive type that correctly uses definitions and references.
#[derive(JsonSchema)]
struct ReferencedNode {
    /// Optional recursive link.
    next: Option<Box<Self>>,
}

/// Non-recursive type that may safely be inlined.
#[derive(JsonSchema)]
#[schemars(inline)]
struct InlineLeaf {
    /// Leaf payload.
    value: String,
}

/// Exercise fixture fields so strict example-target lints stay meaningful.
fn main() {
    let inline = InlineNode { next: None };
    let manual = ManualInlineNode {
        children: Vec::new(),
    };
    let referenced = ReferencedNode { next: None };
    let leaf = InlineLeaf {
        value: String::new(),
    };

    drop((inline.next, manual.children, referenced.next, leaf.value));
}
