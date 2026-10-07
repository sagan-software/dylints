#![feature(rustc_private)]

//! Preserve the text printed by custom Debug builder shapes.
//!
//! A named struct can use a tuple builder, and a tuple struct can use a named
//! builder. Both implementations compile, but neither prints the text that a
//! derived implementation prints. Run the same fixture used by the lint UI
//! test so its expected diagnostic silence protects observable formatting.

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use manual_debug_impl as _;

#[path = "../ui/negative_shapes.rs"]
mod fixture;

/// Custom builder shapes retain their exact named and positional output.
#[test]
fn preserves_custom_builder_output() {
    fixture::main();
}
