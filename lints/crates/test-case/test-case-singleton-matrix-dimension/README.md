# test-case-singleton-matrix-dimension

## What it does

Checks for a `#[test_matrix(...)]` attribute that generates more than one test
and has an input set written as an array or tuple with exactly one value, such
as `["fixed"]` in `#[test_matrix(["fixed"], [0, 1, 2])]`.

## Why is this bad?

A one-value collection makes a constant input look like a varying one. Readers
cannot tell which inputs create the combinations, or whether values were
removed by mistake. test-case accepts a plain expression for a constant input
and generates the same tests. A matrix that generates only one test is covered
by `test-case-single-case-matrix` instead.

## Known problems

The lint checks only array and tuple literals written in the attribute. A
one-value range such as `0..1`, a constant, or a macro does not trigger it. It
gives help text but no automatic fix.

The lint counts every case generated for the function, including cases from
other test-case attributes. It reads only the first test-case attribute on a
function, so a one-value collection in a later attribute is not reported.

## Example

```rust
use test_case::test_matrix;

#[test_matrix(["fixed"], [0, 1, 2])]
fn parses(input: &str, mode: u8) {
    assert!(!input.is_empty() && mode < 3);
}
```

## Use instead

```rust
use test_case::test_matrix;

#[test_matrix("fixed", [0, 1, 2])]
fn parses(input: &str, mode: u8) {
    assert!(!input.is_empty() && mode < 3);
}
```
