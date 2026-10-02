// compile-flags: --edition 2024

#[path = "auxiliary/support.rs"]
mod support;

fn main() {
    support::marker();
}
