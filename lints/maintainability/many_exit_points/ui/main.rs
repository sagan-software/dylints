// compile-flags: --edition 2024

#![allow(dead_code)]

fn exceeds_limit(values: [Option<u8>; 3]) -> Option<u8> {
    let first = values[0]?;
    let second = values[1]?;
    let third = values[2]?;
    if first == 0 {
        return None;
    }
    if second == 0 {
        return None;
    }
    Some(third)
}

fn main() {}
