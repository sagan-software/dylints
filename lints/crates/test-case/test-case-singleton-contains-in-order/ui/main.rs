// compile-flags: --test

use test_case::test_case;

#[test_case(vec![1_u8, 2] => it contains_in_order [2] ; "one element")]
#[test_case(vec![1_u8, 2] => it contains_in_order [1, 2] ; "two elements")]
#[test_case(vec![1_u8, 2] => it contains 2 and contains_in_order [1] ; "joined matcher")]
fn singleton(values: Vec<u8>) -> Vec<u8> {
    values
}

#[test_case(vec![1_u8, 2] => it contains 2 ; "contains element")]
fn direct(values: Vec<u8>) -> Vec<u8> {
    values
}

#[test_case(Some(1_u8) => matches Some(_) ; "simple match")]
fn simple(value: Option<u8>) -> Option<u8> {
    value
}

fn main() {}
