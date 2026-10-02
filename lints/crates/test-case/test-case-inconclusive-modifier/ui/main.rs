// run-rustfix
// rustfix-only-machine-applicable
// compile-flags: --test

use test_case::test_case;

#[test_case(1 => inconclusive["tracked issue"] ; "legacy")]
#[test_case(2 => inconclusive ; "legacy without reason")]
#[test_case(3 => ignore["tracked issue"] ; "direct")]
fn legacy(_value: u8) {}

#[test_case("inconclusive" ; "inconclusive input")]
fn input(_value: &str) {}

fn main() {}
