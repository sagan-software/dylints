# test-case-single-case-matrix

## What it does

Checks for a `#[test_matrix(...)]` function that generates exactly one test,
because every input set has one value.

## Why is this bad?

`test_matrix` says that inputs vary. When the matrix generates one test, the
inputs do not vary, and `#[test_case(...)]` states that shape directly.

## Known problems

The lint counts every case generated for the function. When the function also
has a `#[test_case(...)]` attribute or another `#[test_matrix(...)]`
attribute, the single-case matrix is not reported.

The lint gives help text but no automatic fix.

## Example

```rust
use test_case::test_matrix;

#[test_matrix(["input"], [1])]
fn parses(_input: &str, _expected: u8) {}
```

## Use instead

```rust
use test_case::test_case;

#[test_case("input", 1)]
fn parses(_input: &str, _expected: u8) {}
```
