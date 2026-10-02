// compile-flags: --test

use test_case::test_case;

#[test_case(1 => panics "" ; "empty message")]
#[test_case(2 => panics r"" ; "raw empty message")]
#[test_case(3 => panics " " ; "whitespace message")]
#[test_case(4 => panics "expected" ; "documented")]
fn panicking(_value: u8) {
    panic!("expected");
}

fn main() {}
