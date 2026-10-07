// compile-flags: --edition 2024
#![allow(dead_code)]

// Gates on impl items, trait items, foreign items, variants, and fields all count.
struct Value;

impl Value {
    #[cfg(unix)]
    fn impl_gate(&self) {}

    #[cfg(test)]
    fn test_only_method(&self) {}
}

trait Contract {
    #[cfg(unix)]
    fn trait_gate(&self);

    #[cfg(all(test, unix))]
    fn test_only_trait_item(&self);
}

unsafe extern "C" {
    #[cfg(unix)]
    fn foreign_gate();

    #[cfg(any(test, test))]
    fn test_only_foreign_item();
}

enum Mode {
    #[cfg(unix)]
    Variant,

    #[cfg(test)]
    TestOnlyVariant,

    Other,
}

struct Config {
    #[cfg(unix)]
    field: u8,

    #[cfg(all(test))]
    test_only_field: u8,
}

// Equivalent list predicates count as one predicate.
#[cfg(all(unix, not(windows)))]
const FIRST: u8 = 1;

#[cfg(all(not(windows), unix))]
static SECOND: u8 = 2;

#[cfg(all(unix, not(windows)))]
type Third = u8;

#[cfg(all(unix, not(windows)))]
union Fourth {
    value: u8,
}

// Predicates that do not require `test` keep their gates counted.
#[cfg(any(test, unix))]
use std::fmt as first_fmt;

#[cfg(any(test, unix))]
extern crate core as first_core;

#[cfg(any(test, unix))]
trait FirstAlias {}

#[cfg(any(test, unix))]
macro_rules! first_macro {
    () => {};
}

#[cfg(any())]
mod never {}

#[cfg(all())]
mod always {}

fn main() {}
