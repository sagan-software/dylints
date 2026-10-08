#![allow(rumdl_doc_comments)]
#![deny(unfulfilled_lint_expectations)]

/// Suppressed * emphasis * stays unchecked.
pub fn allowed() {}

#[warn(rumdl_doc_comments)]
mod enabled {
    /// Enabled * emphasis * must still warn inside an allowed crate.
    pub fn warned() {}

    #[allow(rumdl_doc_comments)]
    /// Suppressed * emphasis * stays unchecked inside a warned module.
    pub fn allowed() {}
}

#[expect(
    rumdl_doc_comments,
    reason = "verify expected diagnostics still execute"
)]
/// Expected * emphasis * must fulfill the expectation.
pub fn expected() {}

fn main() {
    allowed();
    enabled::warned();
    enabled::allowed();
    expected();
}
