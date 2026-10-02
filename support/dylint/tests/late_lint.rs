#![feature(rustc_private)]
#![allow(
    unused_crate_dependencies,
    reason = "this test crate imports only the declaration macro under test"
)]

//! Expands `documented_late_lint!` and checks the generated lint-pass metadata.

extern crate rustc_driver as _;

use rustc_lint::{LateLintPass, LintPass as _};

dylint_support::documented_late_lint! {
    /// Demonstration lint used to exercise the generated declaration.
    pub DEMO_LINT,
    Warn,
    "demonstration lint",
    DemoPass
}

impl LateLintPass<'_> for DemoPass {}

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
