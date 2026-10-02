// compile-flags: --test

use test_case::test_case;

#[test_case(1)]
#[test_case(2)]
fn unnamed(_value: u8) {}

fn main() {}
