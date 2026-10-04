//! Compiler UI cases for empty Insta snapshot descriptions.
#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_empty_description as _;
use insta_support as _;

/// An empty current-crate description constant.
const EMPTY: &str = "";

/// Exercise literal, constant, and immutable-local descriptions.
fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_description("");
    settings.set_description(EMPTY);
    settings.set_description("API response");

    let empty_description = "";
    settings.set_description(empty_description);
}
