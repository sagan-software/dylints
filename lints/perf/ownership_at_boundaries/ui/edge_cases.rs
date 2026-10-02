#![allow(dead_code)]

pub fn contains_any(haystack: String, needles: Vec<String>) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub fn stores_owned(value: String) -> String {
    value
}

pub fn mutates_owned(mut values: Vec<String>) -> usize {
    values.push("extra".to_owned());
    values.len()
}

fn main() {}
