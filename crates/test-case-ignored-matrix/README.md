# test-case-ignored-matrix

## What it does

Checks for a `#[test_matrix(...)]` attribute with an `ignore` or
`inconclusive` modifier in its output.

## Why is this bad?

test-case applies the matrix output and its modifiers to every generated case,
so the test harness skips every combination. The matrix looks like broad
coverage while none of it runs.

## Known problems

The lint also warns when a project intentionally skips the whole matrix, for
example during a short migration. An `ignore` modifier without a reason also triggers
`test-case-ignore-without-reason`, and `inconclusive` also triggers
`test-case-inconclusive-modifier`.

## Example

```rust
use test_case::test_matrix;

#[test_matrix([1, 2], [true, false] => ignore["issue #123"])]
fn validates(value: u8, strict: bool) {
    assert!(value > 0 || !strict);
}
```

## Use instead

Remove the modifier. Move inputs that cannot run yet into separate
`#[test_case(...)]` attributes with `ignore["reason"]`.

```rust
use test_case::test_matrix;

#[test_matrix([1, 2], [true, false])]
fn validates(value: u8, strict: bool) {
    assert!(value > 0 || !strict);
}
```
