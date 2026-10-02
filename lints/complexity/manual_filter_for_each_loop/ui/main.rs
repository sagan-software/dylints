fn consume(_value: i32) {}

fn filtered(values: &[i32]) {
    for value in values {
        if *value > 0 {
            consume(*value);
        }
    }
}

fn with_else(values: &[i32]) {
    // Keep quiet when the predicate has an else branch.
    for value in values {
        if *value > 0 {
            consume(*value);
        } else {
            consume(0);
        }
    }
}

fn main() {
    filtered(&[1, -1]);
    with_else(&[1, -1]);
}
