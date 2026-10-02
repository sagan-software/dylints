// compile-flags: --test

use test_case::test_case;

fn validate(actual: u8) {
    assert_eq!(actual, 1);
}

#[test_case(1_u8 => with validate ; "named function")]
fn named(value: u8) -> u8 {
    value
}

#[test_case(2_u8 => with |actual| assert_eq!(actual, 2) ; "inline closure")]
fn inline(value: u8) -> u8 {
    value
}

fn main() {}
