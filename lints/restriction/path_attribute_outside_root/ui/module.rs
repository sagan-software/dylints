// compile-flags: --edition 2024

// A crate root may use `path`, including through `cfg_attr`.
#[path = "auxiliary/support.rs"]
mod direct_support;

#[cfg_attr(all(), path = "auxiliary/support.rs")]
mod conditional_support;

// These files are modules, not crate roots, even when one is named `lib.rs`.
#[path = "auxiliary/nested/outer.rs"]
mod outer;

#[path = "auxiliary/nested/lib.rs"]
mod nested_lib;

fn main() {
    direct_support::marker();
    conditional_support::marker();
    outer::inner::marker();
    nested_lib::inner::marker();
}
