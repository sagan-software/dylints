// compile-flags: --test

use test_case::test_case;

#[test_case(1 ; "")]
fn empty(_value: u8) {}

#[test_case(2 ; "maximum value")]
fn described(_value: u8) {}

fn main() {}
