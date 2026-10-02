// compile-flags: --test

use test_case::test_case;

#[test_case(1 => ignore[""] ; "empty reason")]
fn empty(_value: u8) {}

#[test_case(2 => ignore["tracked issue"] ; "documented")]
fn documented(_value: u8) {}

fn main() {}
