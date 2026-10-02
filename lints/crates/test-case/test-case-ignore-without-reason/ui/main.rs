// compile-flags: --test

use test_case::test_case;

#[test_case(1 => ignore ; "one")]
fn ignored(_value: u8) {}

#[test_case(2 => ignore["tracked issue"] ; "two")]
fn documented(_value: u8) {}

fn main() {}
