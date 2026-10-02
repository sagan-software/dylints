# manual_try_for_each_loop

## What it does

Checks for a `for` loop whose body is one expression with one `?`, when the loop
is the last statement before a `Ok(())`, `Some(())`, or
`ControlFlow::Continue(())` tail.

## Why is this bad?

The loop, the `?`, and the separate success tail spread one fallible pass over
several lines. `Iterator::try_for_each` applies the action, stops at the first
failure, and returns the result in one expression.

## Known problems

- The `?` operand must already have the tail's error, `None`, or break type. A
  `?` that converts the error through `From` is ignored.
- A loop body with more than one statement or more than one `?` is ignored.
- A loop followed by another statement before the tail is ignored.
- The closure can need type annotations, so the lint emits help without an
  automatic fix.

## Example

```rust
use std::io;

fn write_all(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    Ok(())
}
```

## Use instead

```rust
use std::io;

fn write_all(values: &[i32]) -> io::Result<()> {
    values.iter().try_for_each(write_value)
}
```
