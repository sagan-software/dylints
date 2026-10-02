// compile-flags: --test

use test_case::{test_case, test_matrix};

#[test_matrix(0..8, 0..8)]
fn large(_left: u8, _right: u8) {}

#[test_matrix(0..7, 0..9)]
fn just_below_limit(_left: u8, _right: u8) {}

#[test_matrix(0..7, 0..9)]
#[test_case(10, 10)]
fn combined_attributes(_left: u8, _right: u8) {}

fn main() {}
