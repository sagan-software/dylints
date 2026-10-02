// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 => matches Some(_) if true ; "redundant guard")]
fn always_true(value: u8) -> Option<u8> {
    Some(value)
}

#[test_case(2_u8 => matches Some(value) if value > 0 ; "meaningful guard")]
fn meaningful(value: u8) -> Option<u8> {
    Some(value)
}

fn main() {}
