#![allow(dead_code)]

struct Buffer {
    bytes: Vec<u8>,
}

impl Buffer {
    fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    fn blank() -> Self {
        Self { bytes: Vec::new() }
    }

    fn new_with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }
}

fn main() {}
