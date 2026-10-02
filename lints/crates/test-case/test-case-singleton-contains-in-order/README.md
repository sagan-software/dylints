# test-case-singleton-contains-in-order

## What it does

Checks for a `contains_in_order` matcher in a `#[test_case(...)]` or
`#[test_matrix(...)]` output whose expected array or tuple has exactly one
element, such as `contains_in_order [2]`.

## Why is this bad?

Order has no meaning for one element. The case only checks that the element
is present, which `contains` states directly. `contains_in_order` suggests an
order check that does not happen.

## Known problems

The lint checks only an array or tuple literal written in the attribute. A
constant or variable with one element does not trigger it. The lint gives help
text but no automatic fix.

## Example

```rust
use test_case::test_case;

#[test_case(vec![1, 2] => it contains_in_order [2]; "contains two")]
fn values(input: Vec<u8>) -> Vec<u8> {
    input
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(vec![1, 2] => it contains 2; "contains two")]
fn values(input: Vec<u8>) -> Vec<u8> {
    input
}
```
