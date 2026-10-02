#![allow(dead_code)]

fn invalid() {
    insta::glob!("../fixtures/*.txt", |_path| {});
}

fn valid_parent_base() {
    insta::glob!("..", "fixtures/*.txt", |_path| {});
}

fn valid_local_pattern() {
    insta::glob!("fixtures/*.txt", |_path| {});
}

fn main() {}
