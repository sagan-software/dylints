// run-rustfix
// rustfix-only-machine-applicable
fn main() {
    let _ = insta::Settings::new();
    let _ = insta::Settings::clone_current();
}
