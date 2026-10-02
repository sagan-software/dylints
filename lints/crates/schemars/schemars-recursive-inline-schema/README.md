# schemars-recursive-inline-schema

## What it does

Checks for a struct or enum that contains itself and whose `JsonSchema`
implementation returns `true` from `inline_schema`, either through
`#[schemars(inline)]` or a manual implementation.

## Why is this bad?

Schemars documents that `inline_schema` must return `false` for recursive
types. An inlined schema is expanded in place at every use, so a type that
contains itself expands without end when the schema is generated.

## Known problems

The lint only finds a type that contains itself directly, possibly through
generic wrappers such as `Box`, `Option`, and `Vec`, or through arrays, tuples,
and references. It misses mutual recursion through another type, such as `A`
containing `B` and `B` containing `A`. It also misses an `inline_schema` body
that computes its result instead of returning the literal `true`.

## Example

```rust
#[derive(schemars::JsonSchema)]
#[schemars(inline)]
struct Node {
    next: Option<Box<Node>>,
}
```

## Use instead

```rust
#[derive(schemars::JsonSchema)]
struct Node {
    next: Option<Box<Node>>,
}
```
