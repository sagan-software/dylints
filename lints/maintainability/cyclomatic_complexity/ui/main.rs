// compile-flags: --edition 2024

#![allow(dead_code)]

fn exceeds_limit(a: bool, b: bool, c: bool, d: bool, e: bool, f: bool) -> usize {
    let mut score = 0;
    if a {
        score += 1;
    }
    if b {
        score += 1;
    }
    if c {
        score += 1;
    }
    if d {
        score += 1;
    }
    if e {
        score += 1;
    }
    if f {
        score += 1;
    }
    if a && b && c && d && e {
        score += 1;
    }
    score
}

fn main() {}
