# manual_filter_for_each_loop

## What it does

Checks for a `for` loop whose body is only an `if` without `else` or `let`,
where the `if` branch holds one call, method call, or assignment.

## Why is this bad?

Nested blocks handle item selection and the action inside the loop, which adds
two levels of indentation for one condition. `filter(...).for_each(...)` names
the selection and the action as separate steps.

## Known problems

- The lint skips bodies that contain `?`, `.await`, `break`, `continue`, or
  `return` outside a closure.
- Compound assignments such as `total += value` do not count as an action, so
  the lint ignores those loops.
- The lint ignores conditions with `if let` or a let chain.
- Closure arguments can need extra dereferences, such as `**value` for a slice
  iterator. The lint emits help without an automatic fix.

## Example

```rust
# fn consume(value: i32) { let _ = std::hint::black_box(value); }
fn consume_positive(values: &[i32]) {
    for value in values {
        if *value > 0 {
            consume(*value);
        }
    }
}
```

## Use instead

```rust
# fn consume(value: i32) { let _ = std::hint::black_box(value); }
fn consume_positive(values: &[i32]) {
    values
        .iter()
        .filter(|value| **value > 0)
        .for_each(|value| consume(*value));
}
```
