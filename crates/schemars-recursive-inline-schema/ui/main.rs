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
    /// A non-recursive scalar field is checked before the recursive field.
    value: u8,
    /// A non-recursive tuple follows the same field search path.
    metadata: (u8, u16),
    /// Optional recursive link.
    next: Option<Box<Self>>,
}

/// Manually implemented recursive type that reports itself as inline.
struct ManualInlineNode {
    /// Recursive children.
    children: Vec<Self>,
}

/// A tuple wrapper still contains the recursively boxed type.
#[derive(JsonSchema)]
#[schemars(inline)]
struct TupleInlineNode {
    /// Recursive field nested inside a tuple.
    next: (Option<Box<Self>>, u8),
}

/// A recursive reference passes through a reference type constructor.
#[derive(JsonSchema)]
#[schemars(inline)]
struct ReferenceInlineNode {
    /// The reference still reaches the implementing type.
    next: Option<&'static Self>,
}

/// A manual inline policy can return `true` through a block tail.
struct BlockInlineNode {
    /// Recursive children.
    children: Vec<Self>,
}

impl JsonSchema for BlockInlineNode {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "BlockInlineNode".into()
    }

    fn inline_schema() -> bool {
        { true }
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        true.into()
    }
}

/// Recursive type that correctly leaves its schema as a reference.
struct ManualReferencedNode {
    /// Recursive children.
    children: Vec<Self>,
}

impl JsonSchema for ManualReferencedNode {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ManualReferencedNode".into()
    }

    fn inline_schema() -> bool {
        return false;
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        true.into()
    }
}

/// A same-named inherent method does not set the Schemars inline policy.
struct InherentInlineNode {
    /// Recursive children.
    children: Vec<Self>,
}

impl InherentInlineNode {
    fn inline_schema() -> bool {
        true
    }
}

impl InlineLeaf {
    const INLINE_MARKER: bool = true;
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
    let inline = InlineNode {
        value: 0,
        metadata: (0, 0),
        next: None,
    };
    let manual = ManualInlineNode {
        children: Vec::new(),
    };
    let tuple = TupleInlineNode { next: (None, 0) };
    let reference = ReferenceInlineNode { next: None };
    let block = BlockInlineNode {
        children: Vec::new(),
    };
    let manual_referenced = ManualReferencedNode {
        children: Vec::new(),
    };
    let inherent = InherentInlineNode {
        children: Vec::new(),
    };
    let referenced = ReferencedNode { next: None };
    let leaf = InlineLeaf {
        value: String::new(),
    };

    drop((
        inline.next,
        inline.value,
        inline.metadata,
        manual.children,
        tuple.next,
        reference.next,
        block.children,
        manual_referenced.children,
        InherentInlineNode::inline_schema(),
        inherent.children,
        referenced.next,
        leaf.value,
    ));
}
