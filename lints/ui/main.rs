//! The aggregate fixture loads every category through one Dylint library.
//! Its compiler run verifies that category registration does not duplicate lint names.
//! The fixture intentionally contains a source-authored entry point, so registered
//! restriction lints can inspect the same input through the normal compiler interface.

/// Complete the aggregate registration probe.
fn main() {}
