// compile-flags: --test
#![allow(dead_code)]

// Test functions are not called from documentation, so they need no example.
#[test]
fn checks_behavior() {}

mod nested {
    #[test]
    fn nested_check() {}

    // A helper beside a test still needs an example in `all` scope.
    fn helper() {}
}
