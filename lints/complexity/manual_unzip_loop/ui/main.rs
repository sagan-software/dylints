use std::collections::{BTreeSet, VecDeque};

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

fn unzip_tail_insertion(pairs: Vec<(i32, i32)>) -> (BTreeSet<i32>, VecDeque<i32>) {
    // The second insertion can be the loop body's tail expression.
    let mut left = BTreeSet::default();
    let mut right = VecDeque::new();
    for (returned, second) in pairs {
        left.insert(returned);
        right.push_back(second)
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

fn swapped_tail(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    (right, left)
}

fn shadowed(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // The pushed `first` is a new binding, not the tuple field.
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push({
            let first = second;
            first
        });
        right.push(first);
    }
    (left, right)
}

fn transformed(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first + 1);
        right.push(second);
    }
    (left, right)
}

fn rest_pattern(triples: Vec<(i32, i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // A rest pattern skips tuple fields that `unzip` cannot drop.
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, .., third) in triples {
        left.push(first);
        right.push(third);
    }
    (left, right)
}

fn not_tuple_pattern(pairs: Vec<[i32; 2]>) -> (Vec<i32>, Vec<i32>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for [first, second] in pairs {
        left.push(first);
        right.push(second);
    }
    (left, right)
}

fn prefilled(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    let mut left = vec![0];
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    (left, right)
}

fn extra_statement(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
        left.push(first);
    }
    (left, right)
}

fn not_insertion(pairs: Vec<(usize, usize)>) -> (Vec<usize>, Vec<usize>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.truncate(first);
        right.truncate(second);
    }
    (left, right)
}

fn let_statement(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        let _ = first;
        right.push(second);
    }
    (left, right)
}

fn make_vec() -> Vec<i32> {
    Vec::new()
}

fn free_constructor(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // Only an associated `new` or `default` call proves an empty collection.
    let mut left = make_vec();
    let mut right = (|| Vec::new())();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    (left, right)
}

fn statement_window(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    println!("start");
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right = vec![second];
    }
    (left, right)
}

fn not_tuple_tail(pairs: Vec<(i32, i32)>) -> Vec<i32> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    left
}

fn main() {
    let _ = unzip(vec![(1, String::from("a"))]);
    let _ = swapped(vec![(1, 2)]);
    let _ = trait_constructor(vec![(1, 2)]);
}

struct ForeignFactory;

impl ForeignFactory {
    fn new() -> Vec<i32> {
        vec![99]
    }

    fn default() -> Vec<i32> {
        vec![77]
    }
}

trait Factory {
    fn new() -> Self;
}

impl Factory for Vec<i32> {
    fn new() -> Self {
        vec![99]
    }
}

fn foreign_constructor(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet because both associated functions belong to a non-collection owner.
    let mut left: Vec<i32> = ForeignFactory::new();
    let mut right: Vec<i32> = ForeignFactory::default();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    (left, right)
}

fn trait_constructor(pairs: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet because the trait constructor returns a nonempty collection.
    let mut left: Vec<i32> = <Vec<i32> as Factory>::new();
    let mut right = Vec::new();
    for (first, second) in pairs {
        left.push(first);
        right.push(second);
    }
    (left, right)
}
