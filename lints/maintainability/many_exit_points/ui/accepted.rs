// compile-flags: --edition 2024

#![allow(dead_code)]

fn exactly_at_limit(values: [Option<u8>; 2]) -> Option<u8> {
    let first = values[0]?;
    let second = values[1]?;
    if first == 0 {
        return None;
    }
    if second == 0 {
        return None;
    }
    Some(first + second)
}

fn main() {}
