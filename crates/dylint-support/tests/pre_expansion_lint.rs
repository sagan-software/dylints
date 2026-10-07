#![feature(rustc_private)]
#![expect(
    unused_crate_dependencies,
    reason = "this test crate imports only the declaration macro under test"
)]

//! This integration test expands `documented_pre_expansion_lint!` and checks
//! the generated lint declaration before macro expansion. Compilation checks
//! early-pass trait compatibility, while runtime assertions verify the pass name
//! and registered lint name used by the compiler driver. The test keeps this
//! pre-expansion declaration contract visible independently from later passes.

extern crate rustc_driver as _;

use rustc_lint::{EarlyLintPass, LintPass as _};

dylint_support::documented_pre_expansion_lint! {
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
