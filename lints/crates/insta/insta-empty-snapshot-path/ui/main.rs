//! Compiler UI cases for empty Insta snapshot paths.
#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_empty_snapshot_path as _;
use insta_support as _;

/// Exercise literal and immutable-local snapshot paths.
fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("");
    settings.set_snapshot_path("API response");

    let empty_path = "";
    settings.set_snapshot_path(empty_path);
}
