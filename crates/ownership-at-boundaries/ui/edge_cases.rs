// compile-flags: --edition=2024
#![allow(dead_code)]

pub fn contains_any(haystack: String, needles: Vec<String>) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub fn reads_through_closure(value: String) -> impl Fn() -> usize {
    let length = value.len();
    move || length
}

// Ownership is needed when the value is returned, stored, mutated, or consumed.
pub fn stores_owned(value: String) -> String {
    value
}

pub struct Named {
    name: String,
}

impl Named {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

pub fn mutates_owned(mut values: Vec<String>) -> usize {
    values.push("extra".to_owned());
    values.len()
}

pub fn consumes_by_iteration(values: Vec<String>) -> Vec<usize> {
    values.into_iter().map(|value| value.len()).collect()
}

pub fn captured_by_move(value: String) -> impl Fn() -> usize {
    move || value.len()
}

pub fn rebinds_mutably(values: Vec<u8>) -> Vec<u8> {
    let mut values = values;
    values.push(1);
    values
}

pub fn destructured((left, right): (String, String)) -> usize {
    left.len() + right.len()
}

pub fn borrowed_and_counted(value: &str, count: u8) -> usize {
    let mut total = 0;
    total = total + value.len() + usize::from(count);
    total
}

pub fn assigns_locals(value: String) -> usize {
    let mut total = 0;
    total = total + value.len();
    total
}

pub fn element_copy(values: Vec<u8>) -> u8 {
    values[0]
}

// A trait fixes the signature of its methods and their implementations.
pub trait Sink {
    fn accept(&self, value: String) -> usize {
        value.len()
    }
}

pub struct Counter;

impl Sink for Counter {
    fn accept(&self, value: String) -> usize {
        value.len()
    }
}

impl From<Vec<u8>> for Counter {
    fn from(values: Vec<u8>) -> Self {
        let _ = values.len();
        Counter
    }
}

// An `async fn` moves its parameters into the returned future.
pub async fn async_reads(value: String) -> usize {
    value.len()
}

macro_rules! generated {
    () => {
        pub fn generated(value: String) -> usize {
            value.len()
        }
    };
}

generated!();

fn main() {}
