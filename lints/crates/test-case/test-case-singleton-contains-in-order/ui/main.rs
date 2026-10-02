// compile-flags: --test

use test_case::test_case;

#[test_case(vec![1_u8, 2] => it contains_in_order [2] ; "one element")]
fn singleton(values: Vec<u8>) -> Vec<u8> {
    values
}

#[test_case(vec![1_u8, 2] => it contains 2 ; "contains element")]
fn direct(values: Vec<u8>) -> Vec<u8> {
    values
}

fn main() {}
