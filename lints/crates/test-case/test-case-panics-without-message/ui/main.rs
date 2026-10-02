// compile-flags: --test

use test_case::test_case;

#[test_case(1 => panics ; "one")]
fn panics(_value: u8) {
    panic!("expected");
}

#[test_case(2 => panics "expected" ; "two")]
fn documented(_value: u8) {
    panic!("expected");
}

fn main() {}
