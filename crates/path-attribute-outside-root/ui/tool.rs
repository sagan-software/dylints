// compile-flags: --edition 2024

// A binary root such as `src/bin/tool.rs` is a crate root.
#[path = "auxiliary/support.rs"]
mod support;

fn main() {
    support::marker();
}
