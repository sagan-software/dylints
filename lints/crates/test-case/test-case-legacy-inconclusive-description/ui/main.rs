// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 ; "inconclusive - blocked by issue #123")]
#[test_case(4_u8 ; "active case")]
#[test_case(5_u8 ; "inconclusive")]
fn legacy(_value: u8) {}

#[test_case(2_u8 => ignore["blocked by issue #123"] ; "tracked skip")]
fn explicit(_value: u8) {}

#[test_case(3_u8 ; "rejects inconclusive state")]
#[test_case(6_u8 ; "inconclusively decided")]
fn ordinary_description(_value: u8) {}

fn main() {}
