// compile-flags: --test

use test_case::test_case;

#[test_case(0_u8 => panics " " ; "blank expectation")]
#[test_case(1_u8 => panics "" ; "empty expectation")]
#[test_case(2_u8 => panics "expected failure" ; "specific expectation")]
fn blank(_value: u8) {
    panic!("expected failure");
}

#[test_case(0_u8 ; "first")]
#[test_case(1_u8 => panics "\n" ; "later attribute")]
fn later(_value: u8) {
    panic!("expected failure");
}

fn main() {}
