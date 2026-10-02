#![feature(rustc_private)]
#![allow(
    unused_crate_dependencies,
    reason = "this test crate imports only the declaration macro under test"
)]

//! This integration test expands `documented_late_lint_with_pass!` with an explicitly constructed
//! pass. Compilation checks the generated declaration and pass traits, while runtime assertions
//! verify the pass name and registered lint name so callers retain the metadata contract.

extern crate rustc_driver as _;

use rustc_lint::{LateLintPass, LintPass as _};

/// Explicitly constructed pass supplied to the generated declaration. This unit type carries no
/// state and implements `LateLintPass`.
///
/// Its constructor and trait implementation keep the macro's explicit pass registration type
/// checks visible to callers.
#[derive(Clone, Copy, Debug)]
pub struct DemoPass;

dylint_support::documented_late_lint_with_pass! {
    /// Demonstration lint used to exercise the generated declaration.
    pub DEMO_LINT,
    Warn,
    "demonstration lint",
    DemoPass,
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
