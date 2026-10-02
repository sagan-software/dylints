// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 ; "inconclusive - blocked by issue #123")]
fn legacy(_value: u8) {}

#[test_case(2_u8 => ignore["blocked by issue #123"] ; "tracked skip")]
fn explicit(_value: u8) {}

#[test_case(3_u8 ; "rejects inconclusive state")]
fn ordinary_description(_value: u8) {}

fn main() {}
