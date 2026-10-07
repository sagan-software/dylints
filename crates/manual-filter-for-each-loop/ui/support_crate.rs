// compile-flags: --crate-name filter_support

// Internal support crates are skipped.
fn consume(_value: i32) {}

fn filtered(values: &[i32]) {
    for value in values {
        if *value > 0 {
            consume(*value);
        }
    }
}

fn main() {
    filtered(&[1, -1]);
}
