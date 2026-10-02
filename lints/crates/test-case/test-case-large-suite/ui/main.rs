// compile-flags: --test

use test_case::test_matrix;

#[test_matrix(0..8, 0..8)]
fn large(_left: u8, _right: u8) {}

fn main() {}
