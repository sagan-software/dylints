// compile-flags: --test

use test_case::test_case;

#[test_case(0_u8 => panics " " ; "blank expectation")]
fn blank(_value: u8) {
    panic!("expected failure");
}

#[test_case(0_u8 => panics "expected failure" ; "specific expectation")]
fn specific(_value: u8) {
    panic!("expected failure");
}

fn main() {}
