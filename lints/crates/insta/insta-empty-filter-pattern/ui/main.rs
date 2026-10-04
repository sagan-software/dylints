//! Compiler UI cases for empty Insta filter patterns.
#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_empty_filter_pattern as _;
use insta_support as _;

/// Exercise literal and immutable-local filter patterns.
fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter("", "[redacted]");
    settings.add_filter(r"\b[[:xdigit:]]{32}\b", "[id]");

    let empty_pattern = "";
    settings.add_filter(empty_pattern, "[redacted]");
}
