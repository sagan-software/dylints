# interleaved-doc-attributes

## What it does

Warns when a non-documentation attribute appears between documentation attributes on the same item.

## Why is this bad?

Interleaved attributes interrupt the documentation block. Put item attributes after the documentation block.

## Known problems

The lint checks AST attribute order because its subject is source layout. Macro-generated attributes are excluded. A machine-applicable ordering fix is offered only for one interleaved `must_use`, `inline` or `cold` attribute when every attribute is documentation or one of those built-ins. Procedural macros can inspect attribute order, so other cases receive help for manual review.

## Example

```rust
/// Return the input.
#[must_use]
/// The caller retains ownership.
fn identity(value: i32) -> i32 { value }
```

## Use instead

```rust
/// Return the input.
/// The caller retains ownership.
#[must_use]
fn identity(value: i32) -> i32 { value }
```
