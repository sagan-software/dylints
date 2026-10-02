#![allow(dead_code)]

const NAME: &str = "fixed";

fn invalid() {
    for value in 0..2 {
        insta::assert_debug_snapshot!(value);
    }
    for value in 0..2 {
        insta::assert_snapshot!("fixed", value.to_string());
    }
    for value in 0..2 {
        insta::assert_snapshot!(NAME, value.to_string());
    }
    let mut value = 0;
    while value < 2 {
        insta::assert_snapshot!(value.to_string(), @"0");
        value += 1;
    }
    loop {
        insta::assert_binary_snapshot!(".bin", vec![1]);
        break;
    }
}

fn valid_distinct_names() {
    for value in 0..2 {
        insta::assert_snapshot!(format!("case_{value}"), value.to_string());
    }
    for name in ["a.bin", "b.bin"] {
        insta::assert_binary_snapshot!(name, vec![1]);
    }
}

fn valid_duplicates() {
    insta::allow_duplicates! {
        for _ in 0..2 {
            insta::assert_snapshot!("same");
        }
    }
}

fn valid_nested_function() {
    for _ in 0..2 {
        fn helper() {
            insta::assert_snapshot!("once");
        }
        helper();
    }
}

fn valid_outside_loop() {
    insta::assert_snapshot!("once");
}

fn main() {}
