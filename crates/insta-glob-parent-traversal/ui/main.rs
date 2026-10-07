#![allow(dead_code)]

fn invalid() {
    insta::glob!("../fixtures/*.txt", |_path| {});
}

fn invalid_raw_and_bare_parent() {
    insta::glob!(r#"../fixtures/*.txt"#, |_path| {});
    insta::glob!("..", |_path| {});
}

fn valid_parent_base() {
    insta::glob!("..", "fixtures/*.txt", |_path| {});
}

fn valid_local_pattern() {
    insta::glob!("fixtures/*.txt", |_path| {});
}

fn main() {}
