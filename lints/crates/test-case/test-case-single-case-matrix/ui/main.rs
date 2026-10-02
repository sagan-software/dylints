// compile-flags: --test
//! UI fixture for test-case matrices with one-case Cartesian products.
#![expect(
    unused_crate_dependencies,
    reason = "UI examples inherit the lint crate's compiler-only dependencies"
)]

use test_case::test_matrix;

/// Scalar inputs produce one test case.
#[test_matrix("input", 1_u8)]
fn scalar_inputs(_input: &str, _expected: u8) {}

/// Singleton lists produce one test case.
#[test_matrix(["input"], [1_u8])]
fn singleton_lists(_input: &str, _expected: u8) {}

/// A one-value range produces one test case.
#[test_matrix(0..1, [1_u8])]
fn singleton_range(_input: isize, _expected: u8) {}

/// Multiple generated cases remain a meaningful matrix.
#[test_matrix(["a", "b"], [1_u8, 2])]
fn populated_matrix(_input: &str, _expected: u8) {}

fn main() {}
