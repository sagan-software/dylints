// compile-flags: --test

use test_case::test_case;

#[test_case(1 => ignore[""] ; "empty")]
#[test_case(2 => ignore[" "] ; "whitespace only")]
#[test_case(3 => inconclusive[r""] ; "raw legacy spelling")]
#[test_case(4 => ignore["tracked issue"] ; "documented")]
#[test_case(5 ; "ignore[\"\"] in the description")]
fn ignored(_value: u8) {}

fn main() {}
