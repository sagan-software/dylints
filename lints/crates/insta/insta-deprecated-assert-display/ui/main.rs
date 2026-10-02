#![allow(deprecated)]

fn main() {
    insta::assert_display_snapshot!("value");
    insta::assert_snapshot!("value");
}
