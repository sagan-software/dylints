// compile-flags: --test

use test_case::test_matrix;

#[test_matrix([1_u8], [true, false])]
fn singleton_collection(_value: u8, _expected: bool) {}

#[test_matrix(1_u8, [true, false])]
fn scalar_constant(_value: u8, _expected: bool) {}

#[test_matrix([1_u8, 2], [true, false])]
fn varying_dimensions(_value: u8, _expected: bool) {}

fn main() {}
