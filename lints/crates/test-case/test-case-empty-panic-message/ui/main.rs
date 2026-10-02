// compile-flags: --test

use test_case::test_case;

#[test_case(1 => panics "" ; "empty message")]
fn empty(_value: u8) {
    panic!("expected");
}

#[test_case(2 => panics "expected" ; "documented")]
fn documented(_value: u8) {
    panic!("expected");
}

fn main() {}
