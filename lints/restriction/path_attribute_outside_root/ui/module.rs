// compile-flags: --edition 2024

#[path = "auxiliary/support.rs"]
mod direct_support;

#[cfg_attr(all(), path = "auxiliary/support.rs")]
mod conditional_support;

fn main() {
    direct_support::marker();
    conditional_support::marker();
}
