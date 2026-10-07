// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use insta::internals::Content;

fn invalid(content: &Content) -> Option<&str> {
    content.resolve_inner().as_str()
}

fn valid(content: &Content) -> Option<&str> {
    content.as_str()
}

fn main() {}
