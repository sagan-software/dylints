// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 => matches _ ; "vacuous")]
#[test_case(2_u8 => matches Some(_) ; "variant checked")]
#[test_case(3_u8 => matches (_) ; "parenthesized")]
fn wildcard(value: u8) -> Option<u8> {
    Some(value)
}

#[test_case(4_u8 => matches Some(_) ; "variant checked")]
fn variant(value: u8) -> Option<u8> {
    Some(value)
}

fn main() {}
