# manual_iterator_loop

## What it does

Checks for a mutable accumulator declared just before a `for` loop that fills
it in one of four ways:

- `Vec::new()`, then a body that is only `out.push(..)`.
- An integer `0`, then a body that is only `if condition { count += 1 }`.
- `false`, then a body that is only `if condition { found = true }`.
- `true`, then a body that is only `if condition { all_ok = false }`.

## Why is this bad?

The mutable binding, the loop, and the update spread one result over several
lines. The boolean loops also keep iterating after the answer is known.
`map(...).collect()`, `filter(...).count()`, `any(...)`, and `all(...)` compute
the same result in one expression, and `any` and `all` stop at the first
decisive item.

## Known problems

- The accumulator must be declared in the statement directly before the loop.
  A loop separated from its accumulator by another statement is ignored.
- The loop body must be the single push or the single `if` shown above. A
  `push` inside an `if`, a body with a `break`, or a body with extra statements
  is ignored.
- Only `Vec::new()` starts a collection. `vec![]` and `Vec::with_capacity(n)`
  are ignored.
- A `Vec` push loop can also trigger `manual_extend_loop`.

## Example

```rust
fn count_even(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value % 2 == 0 {
            count += 1;
        }
    }
    count
}
```

## Use instead

```rust
fn count_even(values: &[i32]) -> usize {
    values.iter().filter(|value| **value % 2 == 0).count()
}
```
