# manual_adjacent_window_loop

## What it does

Checks for a `for` loop over `0..values.len().saturating_sub(1)` whose body
indexes a slice, array, or `Vec` exactly twice, once with `[index + 1]`.

## Why is this bad?

The loop rebuilds adjacent pairs with index arithmetic. Each index operation
carries a bounds check, and an off-by-one change to the range or an index can
panic or skip a pair. `slice::windows(2)` yields each adjacent pair directly.

## Known problems

- Only the `saturating_sub(1)` bound triggers. A loop over
  `0..values.len() - 1` is ignored.
- The lint matches the `+ 1]` index by source text. It does not check that both
  indexes use the loop variable or the same slice, so it can trigger when the
  two indexes read different slices.
- If the body also uses the index value itself, `windows(2)` needs
  `.enumerate()` to keep it.

## Example

```rust
fn print_pairs(values: &[i32]) {
    for index in 0..values.len().saturating_sub(1) {
        println!("{} {}", values[index], values[index + 1]);
    }
}
```

## Use instead

```rust
fn print_pairs(values: &[i32]) {
    for pair in values.windows(2) {
        println!("{} {}", pair[0], pair[1]);
    }
}
```
