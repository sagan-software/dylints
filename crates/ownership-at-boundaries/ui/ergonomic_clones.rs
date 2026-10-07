#![feature(ergonomic_clones)]
#![allow(dead_code, incomplete_features)]

use std::sync::Arc;

// An ergonomic `.use` clone of another value does not affect the parameter.
pub fn shares(value: String, shared: Arc<u8>) -> usize {
    let copy = shared.use;
    value.len() + usize::from(*copy) + usize::from(*shared)
}

fn main() {}
