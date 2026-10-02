// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 => ignore["  "] ; "blank reason")]
fn blank(_value: u8) {}

#[test_case(2_u8 => ignore["issue #123"] ; "tracked")]
fn documented(_value: u8) {}

fn main() {}
