// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 => ignore["  "] ; "blank reason")]
#[test_case(2_u8 => ignore[""] ; "empty reason")]
#[test_case(3_u8 => ignore["issue #123"] ; "tracked")]
#[test_case(4_u8 => inconclusive["\t"] ; "legacy blank reason")]
fn documented(_value: u8) {}

fn main() {}
