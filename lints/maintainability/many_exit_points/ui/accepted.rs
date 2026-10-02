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

macro_rules! generated_exits {
    ($value:expr) => {
        if $value.is_none() {
            return None;
        }
        let _ = $value?;
    };
}

fn macro_exits_are_opaque(value: Option<u8>) -> Option<u8> {
    generated_exits!(value);
    generated_exits!(value);
    generated_exits!(value);
    Some(0)
}

fn main() {}
