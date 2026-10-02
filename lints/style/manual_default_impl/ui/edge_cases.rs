#![allow(dead_code)]

struct Defaults {
    names: Vec<String>,
    enabled: bool,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            names: Default::default(),
            enabled: bool::default(),
        }
    }
}

struct CustomDefault {
    enabled: bool,
}

impl Default for CustomDefault {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn main() {}
