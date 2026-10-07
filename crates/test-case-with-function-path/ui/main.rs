// run-rustfix
// rustfix-only-machine-applicable
// compile-flags: --test

use test_case::test_case;

fn validate(actual: u8) {
    assert!(actual > 0);
}

mod checks {
    pub fn validate<T: PartialEq + Default + core::fmt::Debug>(actual: T) {
        assert_ne!(actual, T::default());
    }
}

#[test_case(1_u8 => with validate ; "named function")]
#[test_case(2_u8 => with checks::validate::<u8> ; "generic path")]
#[test_case(3_u8 => using validate ; "already using")]
#[test_case(4_u8 => with |actual: u8| assert_eq!(actual, 4) ; "inline closure")]
fn named(value: u8) -> u8 {
    value
}

fn main() {}
