// A file reached through a directory named `support` is still checked.
#[path = "auxiliary/support/../../main.rs"]
mod large_module_fixture;

fn main() {}
