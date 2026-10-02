fn adjacent(values: &[i32]) {
    // Trigger for the non-panicking range with two adjacent slice indexes.
    for index in 0..values.len().saturating_sub(1) {
        println!(
            "{current} {next}",
            current = values[index],
            next = values[index + 1]
        );
    }
}

fn unsafe_bound(values: &[i32]) {
    // Keep quiet when the empty input case can underflow before iteration.
    for index in 0..values.len() - 1 {
        println!(
            "{current} {next}",
            current = values[index],
            next = values[index + 1]
        );
    }
}

fn main() {
    adjacent(&[1, 2]);
    unsafe_bound(&[1, 2]);
}
