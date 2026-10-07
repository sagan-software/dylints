# consecutive-iterator-loops

## What it does

Checks for two adjacent `for` loops that iterate over local bindings or a direct
borrow of one. The loops must bind patterns of the same shape and item type and
have structurally equal bodies. A use of the first loop's binding must match a
use of the second loop's binding at the same pattern position.

## Why is this bad?

The code repeats the same body. A later change to one copy can miss the other,
and the two loops then do different work by accident. `Iterator::chain` states
one ordered sequence and keeps one copy of the body.

## Known problems

- Sources other than a local binding, `&name`, or `&mut name` never trigger.
  Field accesses such as `self.items` and calls such as `list.iter()` are
  ignored.
- The body comparison supports common expression and statement forms. A body
  that uses another form, such as a labeled block, a cast, a struct literal, or
  a `while` loop. The lint ignores that body.
- The lint ignores a body that contains `break`, because after chaining the
  `break` would also skip the second sequence.
- `chain` converts the second source into an iterator before the first loop
  runs. The lint only emits help and does not offer an automatic fix.

## Example

```rust
# fn consume(value: &i32) { let _ = std::hint::black_box(value); }
fn report(first: &[i32], second: &[i32]) {
    for value in first {
        consume(value);
    }
    for value in second {
        consume(value);
    }
}
```

## Use instead

```rust
# fn consume(value: &i32) { let _ = std::hint::black_box(value); }
fn report(first: &[i32], second: &[i32]) {
    for value in first.iter().chain(second) {
        consume(value);
    }
}
```
