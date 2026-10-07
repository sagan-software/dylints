// compile-flags: --test

use test_case::test_case;

#[test_case(1 => panics ; "one")]
#[test_case(2 => panics "expected" ; "two")]
#[test_case(3 => panics)]
fn panics(_value: u8) {
    panic!("expected");
}

#[test_case("panics" ; "panics as input")]
fn input(_value: &str) {}

fn main() {}
