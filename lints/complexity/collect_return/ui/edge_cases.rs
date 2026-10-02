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

pub trait Source {
    fn values(&self) -> Vec<u64>;
}

pub struct Numbers(pub Vec<u64>);

impl Source for Numbers {
    fn values(&self) -> Vec<u64> {
        self.0.iter().copied().collect()
    }
}

mod private {
    pub fn hidden(values: &[u64]) -> Vec<u64> {
        values.iter().copied().collect()
    }
}

pub mod exported {
    pub fn visible(values: &[u64]) -> Vec<u64> {
        values.iter().copied().collect()
    }
}

macro_rules! collecting_fn {
    ($name:ident) => {
        pub fn $name(values: &[u64]) -> Vec<u64> {
            values.iter().copied().collect()
        }
    };
}

collecting_fn!(generated);

fn main() {
    let _ = private::hidden(&[]);
}

pub fn ordered_set(values: &[u64]) -> std::collections::BTreeSet<u64> {
    values.iter().copied().collect()
}

pub fn scalar_count(values: &[u64]) -> usize {
    values.len()
}

pub fn constructed_vector() -> Vec<u64> {
    Vec::new()
}

pub fn early_vector() -> Vec<u64> {
    return Vec::new();
}
