use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

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

fn deques(values: Vec<i32>) -> (VecDeque<i32>, VecDeque<i32>) {
    // Trigger for `VecDeque::push_back` and `Default::default` through the type.
    let mut accepted = VecDeque::default();
    let mut rejected = VecDeque::new();
    for value in values {
        if value > 0 {
            accepted.push_back(value);
        } else {
            rejected.push_back(value);
        }
    }
    (accepted, rejected)
}

fn sets(values: Vec<i32>) -> (HashSet<i32>, HashSet<i32>) {
    // Trigger for `HashSet::insert`.
    let mut accepted = HashSet::new();
    let mut rejected = HashSet::new();
    for value in values {
        if value > 0 {
            accepted.insert(value);
        } else {
            rejected.insert(value);
        }
    }
    (accepted, rejected)
}

fn ordered_sets(values: Vec<i32>) -> (BTreeSet<i32>, BTreeSet<i32>) {
    // Trigger for `BTreeSet::insert` and a plain `Default::default` call.
    let mut accepted: BTreeSet<i32> = Default::default();
    let mut rejected = BTreeSet::new();
    for value in values {
        if value > 0 {
            accepted.insert(value);
        } else {
            rejected.insert(value);
        }
    }
    (accepted, rejected)
}

fn macro_constructor(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Trigger when `vec![]` expands to `Vec::new`.
    let mut accepted = vec![];
    let mut rejected = vec![];
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

const FALLBACK: i32 = 0;

fn other_item(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when a branch inserts a value other than the loop item.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(FALLBACK);
        }
    }
    (accepted, rejected)
}

fn mixed_types(values: Vec<i32>) -> (Vec<i32>, VecDeque<i32>) {
    // Keep quiet when the collections have different types.
    let mut accepted = Vec::new();
    let mut rejected = VecDeque::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push_back(value);
        }
    }
    (accepted, rejected)
}

fn maps(values: Vec<i32>) -> (HashMap<i32, i32>, HashMap<i32, i32>) {
    // Keep quiet for maps, whose insertion takes a key and a value.
    let mut accepted = HashMap::new();
    let mut rejected = HashMap::new();
    for value in values {
        if value > 0 {
            accepted.insert(value, value);
        } else {
            rejected.insert(value, value);
        }
    }
    (accepted, rejected)
}

fn reads_accumulator(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the condition reads an accumulator that `partition` owns.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if accepted.len() < 2 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn captures_accumulator(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when a closure in the condition captures an accumulator.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if (|| rejected.is_empty())() {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn early_exit(values: Vec<Option<i32>>) -> Option<(Vec<Option<i32>>, Vec<Option<i32>>)> {
    // Keep quiet when the condition can return from the function.
    let pair = {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for value in values {
            if value? > 0 {
                accepted.push(value);
            } else {
                rejected.push(value);
            }
        }
        (accepted, rejected)
    };
    Some(pair)
}

fn swapped_targets(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the true branch fills the second tuple element.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            rejected.push(value);
        } else {
            accepted.push(value);
        }
    }
    (accepted, rejected)
}

fn indexed_insert(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet for insertion methods with more than one argument.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.insert(0, value);
        } else {
            rejected.insert(0, value);
        }
    }
    (accepted, rejected)
}

fn wrong_method(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet for a single-argument method that is not the collection's insertion.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.truncate(value as usize);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn tuple_pattern(values: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the loop pattern destructures the item.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for (value, _) in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn extra_statement(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the body or a branch does more than one insertion.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn two_body_statements(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the loop body has more than the conditional.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        let copy = value;
        if copy > 0 {
            accepted.push(copy);
        } else {
            rejected.push(copy);
        }
    }
    (accepted, rejected)
}

fn without_else(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet for a conditional without an `else` branch.
    let mut accepted = Vec::new();
    let rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        }
    }
    (accepted, rejected)
}

fn else_if(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet for an `else if` chain.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else if value < 0 {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn iterator_call(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the statement after the accumulators is not a `for` loop.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    values.into_iter().for_each(|value| {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    });
    (accepted, rejected)
}

fn reversed_tail(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the tuple tail swaps the accumulators.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (rejected, accepted)
}

fn empty_vec() -> Vec<i32> {
    Vec::new()
}

trait Maker {}

impl dyn Maker {
    fn new() -> Vec<i32> {
        Vec::new()
    }
}

fn other_constructors(values: Vec<i32>, existing: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet for constructors that are not known to return an empty collection.
    let mut moved = existing;
    let mut called = (|| Vec::new())();
    let mut foreign = <dyn Maker>::new();
    let mut accepted = Vec::with_capacity(4);
    let mut rejected = empty_vec();
    let mut spare = Vec::new();
    let mut other = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
            moved.push(value);
            called.push(value);
            foreign.push(value);
            spare.push(value);
        } else {
            rejected.push(value);
            other.push(value);
        }
    }
    (accepted, rejected)
}

fn single_result(values: Vec<i32>) -> Vec<i32> {
    // Keep quiet when the block does not return both collections.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    accepted
}

struct Bag(Vec<i32>);

impl Bag {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, value: i32) {
        self.0.push(value);
    }
}

fn lookalike(values: Vec<i32>) -> (Bag, Bag) {
    // Keep quiet for a local type with the same method names.
    let mut accepted = Bag::new();
    let mut rejected = Bag::new();
    for value in values {
        if value > 0 {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    (accepted, rejected)
}

fn note() {}

fn immutable(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when a binding before the loop is not a mutable collection.
    note();
    let mut count = 0;
    let accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        count += 1;
        rejected.push(value);
    }
    (accepted, rejected)
}

fn always_push(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    // Keep quiet when the loop body is not a conditional.
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for value in values {
        accepted.push(value);
    }
    rejected.push(0);
    (accepted, rejected)
}

fn main() {
    let _ = partition(vec![1, -1]);
    let _ = deques(vec![1, -1]);
    let _ = sets(vec![1, -1]);
    let _ = ordered_sets(vec![1, -1]);
    let _ = macro_constructor(vec![1, -1]);
    let _ = transformed(vec![1, -1]);
    let _ = other_item(vec![1, -1]);
    let _ = mixed_types(vec![1, -1]);
    let _ = maps(vec![1, -1]);
    let _ = reads_accumulator(vec![1, -1]);
    let _ = captures_accumulator(vec![1, -1]);
    let _ = early_exit(vec![Some(1)]);
    let _ = swapped_targets(vec![1, -1]);
    let _ = indexed_insert(vec![1, -1]);
    let _ = wrong_method(vec![1, -1]);
    let _ = tuple_pattern(vec![(1, 2)]);
    let _ = extra_statement(vec![1, -1]);
    let _ = two_body_statements(vec![1, -1]);
    let _ = without_else(vec![1, -1]);
    let _ = else_if(vec![1, -1]);
    let _ = iterator_call(vec![1, -1]);
    let _ = reversed_tail(vec![1, -1]);
    let _ = other_constructors(vec![1, -1], vec![]);
    let _ = single_result(vec![1, -1]);
    let _ = lookalike(vec![1, -1]);
    let _ = immutable(vec![1, -1]);
    let _ = always_push(vec![1, -1]);
}
