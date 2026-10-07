// compile-flags: --test

use test_case::{test_case, test_matrix};

#[test_matrix([1_u8, 2] => ignore["tracked issue"] ; "disabled")]
fn ignored(_value: u8) {}

#[test_matrix([1_u8, 2] ; "active")]
fn active(_value: u8) {}

#[test_case(0_u8 => ignore["one case"] ; "single case")]
#[test_matrix([1_u8, 2] => inconclusive ; "later matrix")]
fn later(_value: u8) {}

fn main() {}
