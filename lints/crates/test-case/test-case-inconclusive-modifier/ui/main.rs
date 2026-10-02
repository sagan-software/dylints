// compile-flags: --test

use test_case::test_case;

#[test_case(1 => inconclusive["tracked issue"] ; "legacy")]
fn legacy(_value: u8) {}

#[test_case(2 => ignore["tracked issue"] ; "direct")]
fn direct(_value: u8) {}

fn main() {}
