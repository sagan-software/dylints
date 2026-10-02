// compile-flags: --edition=2024
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

struct Action;
struct Error;

fn bool_first() -> (bool, Vec<Action>) {
    (false, Vec::new())
}

fn optional_queue() -> Option<(VecDeque<Action>, bool)> {
    None
}

fn array_pair() -> ([u8; 4], bool) {
    ([0; 4], true)
}

fn hash_map() -> Result<(HashMap<u8, u8>, bool), Error> {
    Err(Error)
}

fn hash_set() -> Result<(HashSet<u8>, bool), Error> {
    Err(Error)
}

fn btree_map() -> Result<(BTreeMap<u8, u8>, bool), Error> {
    Err(Error)
}

fn btree_set() -> Result<(BTreeSet<u8>, bool), Error> {
    Err(Error)
}

async fn async_pair() -> (Vec<Action>, bool) {
    (Vec::new(), false)
}

async fn async_plain() -> Vec<Action> {
    Vec::new()
}

mod lookalike {
    pub struct Vec<T>(pub T);

    pub fn local_vec() -> (Vec<u8>, bool) {
        (Vec(0), false)
    }
}

fn three_elements() -> (Vec<Action>, bool, bool) {
    (Vec::new(), false, false)
}

fn borrowed_slice(values: &[Action]) -> (&[Action], bool) {
    (values, false)
}

fn no_return() {}

fn main() {
    let _ = || (Vec::<u8>::new(), true);
}
