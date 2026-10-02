# manual_adjacent_window_loop

## What it does

Checks for a `for index in 0..values.len().saturating_sub(1)` loop over a
slice, array, or `Vec` binding whose body uses `index` only in `values[index]`
and `values[index + 1]`. It suggests `for window in values.windows(2)` with
`window[0]` and `window[1]`.

## Why is this bad?

The loop rebuilds adjacent pairs with index arithmetic. Each index operation
carries a bounds check, and an off-by-one change to the range or an index can
panic or skip a pair. `slice::windows(2)` yields each adjacent pair directly.

## Known problems

- Only the `saturating_sub(1)` bound triggers. A loop over
  `0..values.len() - 1` is ignored.
- The sequence must be a built-in slice, array, or `Vec` reached from an
  immutable local binding through only built-in references. Custom `Deref`
  receivers and fields such as `self.values` are ignored.
- A body that uses `index` in any other way, reads another element of the
  slice, or already uses the name `window` is ignored.

## Example

```rust
fn print_pairs(values: &[i32]) {
    for index in 0..values.len().saturating_sub(1) {
        let current = values[index];
        let next = values[index + 1];
        println!("{current} {next}");
    }
}
```

## Use instead

```rust
fn print_pairs(values: &[i32]) {
    for window in values.windows(2) {
        let current = window[0];
        let next = window[1];
        println!("{current} {next}");
    }
}
```
