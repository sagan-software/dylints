// compile-flags: --test

use test_case::test_case;

#[test_case(1 => ignore ; "one")]
#[test_case(2 => ignore["tracked issue"] ; "two")]
#[test_case(3 => ignore ["spaced reason"] ; "three")]
#[test_case(4 => ignore)]
#[test_case(5 => inconclusive ; "legacy spelling")]
fn ignored(_value: u8) {}

#[test_case("ignore me" ; "string input")]
#[test_case("other" ; "ignore in the description")]
fn input(_value: &str) {}

fn main() {}
