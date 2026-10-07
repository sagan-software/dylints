# manual-try-for-each-loop

## What it does

Checks for a `for` loop whose body is one expression with one `?`. The loop must
be the last statement of a function, closure, or async body, and the body must
end with `Ok(())`, `Some(())`, or `ControlFlow::Continue(())`.

## Why is this bad?

The loop, the `?`, and the separate success tail spread one fallible pass over
several lines. `Iterator::try_for_each` applies the action, stops at the first
failure, and returns the result in one expression.

## Known problems

- The `?` operand must already have the tail's error, `None`, or break type. The
  lint ignores a `?` that converts the error through `From`.
- The lint ignores a loop body with more than one statement or more than one
  `?`. It also ignores a body that contains `break`, `continue`, `return`, or `.await`,
  because those would apply to the closure.
- The lint ignores a loop followed by another statement before the tail. It also
  ignores a loop in a nested block, because its `?` returns from the enclosing
  body.
- The lint ignores a tail produced by a macro.
- The closure can need type annotations, so the lint emits help without an
  automatic fix.

## Example

```rust
# fn write_value(value: &i32) -> std::io::Result<()> { use std::io::Write as _; writeln!(std::io::sink(), "{value}") }
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
# fn write_value(value: &i32) -> std::io::Result<()> { use std::io::Write as _; writeln!(std::io::sink(), "{value}") }
use std::io;

fn write_all(values: &[i32]) -> io::Result<()> {
    values.iter().try_for_each(write_value)
}
```
