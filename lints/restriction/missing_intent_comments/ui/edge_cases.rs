#![allow(dead_code)]

fn branchy_without_comment(value: u64, fallback: u64) -> u64 {
    let normalized = value.saturating_add(1);
    if normalized > 10 {
        normalized / 2
    } else {
        fallback.saturating_sub(normalized)
    }
}

fn branchy_with_comment(value: u64, fallback: u64) -> u64 {
    // Clamp the fast path before subtracting so callers never see underflow behavior.
    let normalized = value.saturating_add(1);
    if normalized > 10 {
        normalized / 2
    } else {
        fallback.saturating_sub(normalized)
    }
}

fn tiny(value: u64) -> u64 {
    value + 1
}

fn main() {}
