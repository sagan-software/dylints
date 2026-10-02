// compile-flags: --test
//! UI fixture for test-case matrices with empty Cartesian products.
#![expect(
    unused_crate_dependencies,
    reason = "UI examples inherit the lint crate's compiler-only dependencies"
)]

use test_case::test_matrix;

/// Empty list produces no test cases.
#[test_matrix([], [1_u8, 2])]
fn empty_list(_input: &str, _expected: u8) {}

/// Empty half-open range produces no test cases.
#[test_matrix(0..0, [1_u8, 2])]
fn empty_range(_input: isize, _expected: u8) {}

/// Nonempty Cartesian product remains valid.
#[test_matrix(["a", "b"], [1_u8, 2])]
fn populated_matrix(_input: &str, _expected: u8) {}

fn main() {}
