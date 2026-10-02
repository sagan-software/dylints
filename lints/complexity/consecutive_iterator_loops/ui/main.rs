fn consume(_value: &i32) {}

fn chained(first: &[i32], second: &[i32]) {
    // Trigger when adjacent loops have the same local source and body shape.
    for value in first {
        consume(value);
    }
    for value in second {
        consume(value);
    }
}

fn different(first: &[i32], second: &[i32]) {
    // Keep quiet when the second loop has an additional action.
    for value in first {
        consume(value);
    }
    for value in second {
        consume(value);
        consume(value);
    }
}

fn main() {
    chained(&[1], &[2]);
    different(&[1], &[2]);
}
