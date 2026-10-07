//! Compiler UI cases for binary snapshot extension validation.
#![feature(rustc_private)]
#![allow(
    dead_code,
    reason = "compiler UI fixture items are intentionally not executed"
)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_binary_snapshot_missing_extension as _;
use insta_support as _;

/// Exercise a binary snapshot name without an extension.
fn invalid() {
    insta::assert_binary_snapshot!("response", Vec::new());
}

/// A current-crate name without a binary extension.
const NAME: &str = "constant";

/// Exercise a current-crate constant missing its extension.
fn invalid_constant() {
    insta::assert_binary_snapshot!(NAME, Vec::new());
}

/// Pass a missing-extension name through two immutable local bindings.
fn invalid_local_name() {
    let base = "response";
    let name = base;
    insta::assert_binary_snapshot!(name, Vec::new());
}

/// Keep a named binary snapshot with its extension.
fn valid_named() {
    insta::assert_binary_snapshot!("response.bin", Vec::new());
}

/// Keep Insta’s implicit binary snapshot name.
fn valid_implicit() {
    insta::assert_binary_snapshot!(".bin", Vec::new());
}

/// Keep a valid extension when the snapshot name comes from an immutable local.
fn valid_local_extension() {
    let name = "response.bin";
    insta::assert_binary_snapshot!(name, Vec::new());
}

/// Leave a caller-supplied binary snapshot name unknown.
fn dynamic_name_is_not_evaluated(name: &str) {
    insta::assert_binary_snapshot!(name, Vec::new());
}

/// Provide the binary entry point required by the UI example.
fn main() {}
