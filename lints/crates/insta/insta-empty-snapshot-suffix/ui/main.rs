//! Compiler UI cases for empty Insta snapshot suffixes.
#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_empty_snapshot_suffix as _;
use insta_support as _;

/// Exercise literal and immutable-local snapshot suffixes.
fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_suffix("");
    settings.set_snapshot_suffix("API response");

    let empty_suffix = "";
    settings.set_snapshot_suffix(empty_suffix);
}
