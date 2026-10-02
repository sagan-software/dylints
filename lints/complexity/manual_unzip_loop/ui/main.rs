fn unzip(pairs: Vec<(i32, String)>) -> (Vec<i32>, Vec<String>) {
    // Trigger for ordered direct insertion of both tuple fields.
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    (left, right)
}

fn swapped(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when tuple fields are inserted in the opposite order.
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(second);
        right.push(first);
    }
    (left, right)
}

fn main() {
    let _ = unzip(vec![(1, String::from("a"))]);
    let _ = swapped(vec![(1, 2)]);
}
