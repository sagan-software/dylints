// compile-flags: --test

use test_case::test_case;

#[test_case(1 ; "")]
fn empty(_value: u8) {}

#[test_case(1 ; r"")]
fn raw(_value: u8) {}

#[test_case(1 ; "first value")]
#[test_case(2 ; "")]
fn later_attribute(_value: u8) {}

#[test_case(2 ; "maximum value")]
#[test_case("" ; "empty input")]
fn described<T>(_value: T) {}

fn main() {}
