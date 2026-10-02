fn partition(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Trigger for opposite direct insertions of the same item.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn transformed(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when one branch transforms the inserted item.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value * 2);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn main() {
    let _ = partition(vec![1, -1]);
    let _ = transformed(vec![1, -1]);
}
