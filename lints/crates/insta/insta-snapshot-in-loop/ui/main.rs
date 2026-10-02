#![allow(dead_code)]

fn invalid() {
    for value in 0..2 {
        insta::assert_debug_snapshot!(value);
    }
}

fn valid_duplicates() {
    insta::allow_duplicates! {
        for _ in 0..2 {
            insta::assert_snapshot!("same");
        }
    }
}

fn valid_outside_loop() {
    insta::assert_snapshot!("once");
}

fn main() {}
