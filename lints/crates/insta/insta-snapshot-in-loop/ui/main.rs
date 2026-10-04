//! Compiler UI cases for repeated Insta snapshot assertions in loops.
#![feature(rustc_private)]
#![allow(
    dead_code,
    reason = "compiler UI fixture items are intentionally not executed"
)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_snapshot_in_loop as _;
use insta_support as _;

/// A repeated current-crate snapshot name.
const NAME: &str = "fixed";

/// Exercise repeated snapshot names in for, while, and bare loops.
#[expect(clippy::never_loop, reason = "Exercises a snapshot in a bare loop")]
fn invalid() {
    for value in 0..2 {
        insta::assert_debug_snapshot!(value);
    }
    for value in 0..2 {
        insta::assert_snapshot!("fixed", value.to_string());
    }
    for value in 0..2 {
        insta::assert_snapshot!(NAME, value.to_string());
    }
    let mut value = 0;
    while value < 2 {
        insta::assert_snapshot!(value.to_string(), @"0");
        value += 1;
    }
    loop {
        insta::assert_binary_snapshot!(".bin", vec![1]);
        break;
    }
}

/// Keep a literal local name from repeating across loop iterations.
fn invalid_immutable_local_name() {
    let name = "fixed local";
    for value in 0..2 {
        insta::assert_snapshot!(name, value.to_string());
    }
}

/// Use different computed names on each pass through every loop.
fn valid_distinct_names() {
    for value in 0..2 {
        insta::assert_snapshot!(format!("case_{value}"), value.to_string());
    }
    for name in ["a.bin", "b.bin"] {
        insta::assert_binary_snapshot!(name, vec![1]);
    }
}

/// Allow explicitly repeated names through the Insta wrapper.
fn valid_duplicates() {
    insta::allow_duplicates! {
        for _ in 0..2 {
            insta::assert_snapshot!("same");
        }
    }
}

/// Keep a nested function outside the surrounding loop body semantics.
fn valid_nested_function() {
    for _ in 0..2 {
        fn helper() {
            insta::assert_snapshot!("once");
        }
        helper();
    }
}

/// Keep a snapshot assertion outside every loop.
fn valid_outside_loop() {
    insta::assert_snapshot!("once");
}

/// Provide the binary entry point required by the UI example.
fn main() {}
