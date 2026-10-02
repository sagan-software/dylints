// compile-flags: --test

use test_case::test_matrix;

#[test_matrix([1_u8, 2] => ignore["tracked issue"] ; "disabled")]
fn ignored(_value: u8) {}

#[test_matrix([1_u8, 2] ; "active")]
fn active(_value: u8) {}

fn main() {}
