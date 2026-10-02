// run-rustfix
// rustfix-only-machine-applicable
#![allow(deprecated)]

use insta::assert_display_snapshot;

fn main() {
    insta::assert_display_snapshot!("value");
    ::insta::assert_display_snapshot!("value");
    assert_display_snapshot!("value");
    insta::assert_snapshot!("value");
}
