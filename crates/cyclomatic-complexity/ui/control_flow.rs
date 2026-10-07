#![allow(dead_code)]

fn control_flow_kinds(input: Option<u8>, a: bool, b: bool) -> Option<u8> {
    let value = input?;
    while false {}
    for _ in 0..1 {}
    let result = match value {
        0 if a && b => 0,
        1 if a => 1,
        2 => 2,
        3 => 3,
        _ => 4,
    };
    Some(result)
}

fn main() {}
