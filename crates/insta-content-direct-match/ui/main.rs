#![allow(dead_code)]

use insta::internals::Content;

fn invalid(content: &Content) -> bool {
    match content {
        Content::String(_) => true,
        _ => false,
    }
}

fn valid_accessor(content: &Content) -> bool {
    content.as_str().is_some()
}

fn valid_match(content: &Content) -> bool {
    match content.resolve_inner() {
        Content::String(_) => true,
        _ => false,
    }
}

fn main() {}
