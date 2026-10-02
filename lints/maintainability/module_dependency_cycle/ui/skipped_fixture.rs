#![allow(dead_code)]

mod alpha {
    pub fn first() {
        crate::beta::second();
    }
    pub fn leaf() {}
}

mod beta {
    pub fn second() {
        crate::alpha::leaf();
    }
}

fn main() {}
