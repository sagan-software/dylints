#![allow(dead_code)]

fn invalid() {
    insta::assert_binary_snapshot!("response", Vec::new());
}

const NAME: &str = "constant";

fn invalid_constant() {
    insta::assert_binary_snapshot!(NAME, Vec::new());
}

fn valid_named() {
    insta::assert_binary_snapshot!("response.bin", Vec::new());
}

fn valid_implicit() {
    insta::assert_binary_snapshot!(".bin", Vec::new());
}

fn dynamic_name_is_not_evaluated(name: &str) {
    insta::assert_binary_snapshot!(name, Vec::new());
}

fn main() {}
