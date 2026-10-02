// compile-flags: --test
#![allow(dead_code)]

#[test]
fn many_assertions_with_messages() {
    let report = "alpha beta gamma delta epsilon";

    // Message arguments must not hide standard assertion invocations.
    assert!(report.contains("alpha"), "missing alpha");
    assert!(report.contains("beta"), "missing beta");
    assert!(report.contains("gamma"), "missing gamma");
    assert!(report.contains("delta"), "missing delta");
    assert!(report.contains("epsilon"), "missing epsilon");
}

#[test]
fn focused_assertions_are_ok() {
    assert_eq!(2 + 2, 4);
    assert_ne!(1, 2);
}

use std::assert_eq as check_eq;

#[test]
fn aliased_standard_assertions_are_counted() {
    check_eq!(1, 1);
    check_eq!(2, 2);
    check_eq!(3, 3);
    check_eq!(4, 4);
}

#[test]
fn assertion_text_is_not_counted() {
    // Source-like strings must not be mistaken for macro invocations.
    let snippets = [
        "assert!(condition)",
        "assert_eq!(actual, expected)",
        "assert_ne!(actual, unexpected)",
        "debug_assert!(condition)",
    ];

    assert_eq!(snippets.len(), 4);
}

#[test]
fn three_assertions_stay_below_the_threshold() {
    assert!(true);
    assert_eq!(1, 1);
    assert_ne!(1, 2);
}

mod local_macro {
    macro_rules! assert_eq {
        ($left:expr, $right:expr) => {
            let _pair = (&$left, &$right);
        };
    }

    #[test]
    fn lookalike_assertion_macro_is_not_counted() {
        assert_eq!(1, 1);
        assert_eq!(2, 2);
        assert_eq!(3, 3);
        assert_eq!(4, 4);
    }
}

fn main() {}
