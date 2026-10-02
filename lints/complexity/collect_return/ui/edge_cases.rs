#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};

pub fn deque(values: &[u64]) -> VecDeque<u64> {
    values.iter().copied().collect()
}

pub fn map_from_pairs<'a>(values: &'a [(&'a str, u64)]) -> BTreeMap<&'a str, u64> {
    values.iter().copied().collect()
}

pub fn explicit_collection(values: &[u64]) -> Vec<u64> {
    values.iter().copied().collect::<Vec<_>>()
}

fn not_tail(values: &[u64]) -> usize {
    let collected: Vec<_> = values.iter().collect();
    collected.len()
}

fn main() {}
