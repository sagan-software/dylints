# manual_fallible_collect_loop

## What it does

Checks for a `for` loop that pushes or inserts one `?` expression.
The loop must target an empty mutable collection declared just before it.
The block must return that collection as `Ok(collection)` or `Some(collection)`.

## Why is this bad?

The mutable accumulator, the loop, and the wrapped return spread one operation
over several statements. `collect` into `Result<Vec<_>, _>` or
`Option<Vec<_>>` builds the collection and stops at the first failure in one
expression.

## Known problems

- The collection must be a standard `Vec`, `VecDeque`, `HashSet`, `BTreeSet`,
  `HashMap`, or `BTreeMap` created by an argument-free `new` or `default` call.
  The constructor must resolve to that collection's own method or its standard
  `Default` implementation. The lint ignores lookalike associated functions,
  `vec![]`, and `Vec::with_capacity(n)`.
- The loop body must be one `push`, `push_back`, or single-argument `insert`
  call. The lint ignores a body with any other statement or a two-argument
  `HashMap::insert`.
- The inserted expression must contain exactly one `?` operator outside a
  closure, and no `return`, `break`, `continue`, or `.await`. For `Result`, the
  failing expression must already have the function's error type. The lint
  ignores a `?` that converts the error through `From`.
- `collect` can need a type annotation, so the lint emits help without an
  automatic fix.

## Example

```rust
use std::num::ParseIntError;

fn parse_all(values: &[&str]) -> Result<Vec<i32>, ParseIntError> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse()?);
    }
    Ok(output)
}
```

## Use instead

```rust
use std::num::ParseIntError;

fn parse_all(values: &[&str]) -> Result<Vec<i32>, ParseIntError> {
    values.iter().map(|value| value.parse()).collect()
}
```
