// compile-flags: --test

use test_case::test_case;

#[test_case(1)]
#[test_case(2)]
fn unnamed(_value: u8) {}

#[test_case([0; 3])]
#[test_case([1; 3] ; "described repeat")]
fn repeated(_values: [u8; 3]) {}

#[test_case(1 => 1 ; "described")]
fn described(value: u8) -> u8 {
    value
}

fn main() {}
