#![allow(dead_code)]

// Each assertion is one written statement, not one per expanded statement.
fn assertions_without_comments(left: u64, right: u64) {
    assert_eq!(left, right);
    assert_ne!(left, right + 1);
    assert!(left <= right);
}

// Five written macro statements still need a comment.
fn five_prints_without_comments(value: u64) {
    println!("{value}");
    println!("{value}");
    println!("{value}");
    println!("{value}");
    println!("{value}");
}

// Statements written inside a macro argument still count.
fn statements_inside_macro_argument(value: u64) -> Vec<u64> {
    vec![{
        let first = value + 1;
        let second = first + 1;
        let third = second + 1;
        let fourth = third + 1;
        fourth + 1
    }]
}

fn main() {}
