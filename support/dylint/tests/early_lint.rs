#![feature(rustc_private)]
#![allow(
    unused_crate_dependencies,
    reason = "this test crate imports only the declaration macro under test"
)]

//! Expands `documented_early_lint!` and checks the generated lint-pass metadata.

extern crate rustc_driver as _;

use rustc_lint::{EarlyLintPass, LintPass as _};

dylint_support::documented_early_lint! {
    /// Demonstration lint used to exercise the generated declaration.
    pub DEMO_LINT,
    Warn,
    "demonstration lint",
    DemoPass
}

impl EarlyLintPass for DemoPass {}

/// The generated pass reports its type name and its single lint.
#[test]
fn generated_pass_describes_its_lint() {
    let lints = DemoPass.get_lints();

    assert_eq!(DemoPass.name(), "DemoPass");
    assert_eq!(
        lints
            .iter()
            .map(|lint| lint.name_lower())
            .collect::<Vec<_>>(),
        ["demo_lint"]
    );
}
