#![allow(dead_code)]

pub struct EdgeUnused {
    value: usize,
}

pub enum EdgeUnusedEnum {
    Ready,
}

pub type EdgeUnusedAlias = u64;

pub struct EdgeUsed {
    value: usize,
}

fn consume(value: EdgeUsed) -> usize {
    value.value
}

fn main() {}
