# one_use_predicate_binding

## What it does

Checks for an immutable `bool` binding with a generic predicate name, such as
`is_empty` or `has_items`, whose only use is as the condition (or negated
condition) of the `if` in the next statement.

## Why is this bad?

The binding repeats what the expression already says, and the reader must look
one line up to see what the `if` tests. Writing the expression in the `if`
keeps the condition where the branch happens.

## Known problems

- The name check is a fixed word list. A name triggers only when it starts with
  `is`, `has`, `have`, `can`, `should`, `contains`, or `matches` and every
  following word is generic, such as `empty`, `valid`, `ready`, `items`, or
  `value`. A name such as `can_release_funds` or `is_over_limit` is treated as a
  domain concept and ignored, even when it adds no meaning.
- The initializer must be a comparison, a call to a function or method whose
  name starts with `is_`, `has_`, `can_`, or `should_` (or is `contains`,
  `matches`, `starts_with`, `ends_with`, and similar), or a `!`, `&&`, or `||`
  combination of those.
- Uses inside closures, array and tuple literals, and some macro arguments are
  not counted. The lint can trigger when the binding is also used there, and
  inlining it would then break that use.
- `while` conditions are ignored because inlining would re-evaluate the
  expression on every iteration.

## Example

```rust
fn report(values: &[i32]) {
    let is_empty = values.is_empty();
    if is_empty {
        println!("no values");
    }
}
```

## Use instead

```rust
fn report(values: &[i32]) {
    if values.is_empty() {
        println!("no values");
    }
}
```
