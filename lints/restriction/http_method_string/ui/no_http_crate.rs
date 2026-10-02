#![allow(dead_code)]

// Without an HTTP library, `method` and `verb` are generic words.
fn command_method(method: &str, verb: String) -> usize {
    method.len() + verb.len()
}

struct Reflection {
    method: String,
    http_method: String,
}

fn main() {}
