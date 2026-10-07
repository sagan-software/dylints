fn consume(_value: &i32) {}

fn show<T: std::fmt::Debug>(_value: T) {}

fn chained(first: &[i32], second: &[i32]) {
    // Trigger when adjacent loops have local sources and equal bodies.
    for value in first {
        consume(value);
    }
    for value in second {
        consume(value);
    }
}

fn renamed(first: &[i32], second: &[i32]) {
    // Trigger when each body uses its own binding at the same pattern position.
    for left in first {
        consume(left);
    }
    for right in second {
        consume(right);
    }
}

fn tail_loop(first: Vec<i32>, second: Vec<i32>) {
    // Trigger for owned sources when the second loop is the block tail.
    for value in first {
        consume(&value);
    }
    for value in second {
        consume(&value);
    }
}

fn mutable_borrow(mut first: Vec<i32>, mut second: Vec<i32>) {
    // Trigger for mutable borrows of local sources.
    for value in &mut first {
        *value += 1;
    }
    for value in &mut second {
        *value += 1;
    }
}

fn rich(first: &[(i32, Option<i32>)], second: &[(i32, Option<i32>)]) -> i32 {
    // Trigger when every supported expression and statement form corresponds.
    let mut total = 0;
    for &(number, ref maybe) in first {
        let doubled = number * 2;
        total += doubled;
        total = -total;
        let pair = (doubled, [number, 1]);
        consume(&pair.0);
        consume(&pair.1[0]);
        if total > 100 {
            return total;
        } else {
            consume(&total);
        }
        match maybe {
            Some(extra) => consume(extra),
            _ => continue,
        }
        let bumped = maybe.as_ref().map(|value| *value + 1);
        consume(&bumped.unwrap_or(0));
    }
    for &(number, ref maybe) in second {
        let doubled = number * 2;
        total += doubled;
        total = -total;
        let pair = (doubled, [number, 1]);
        consume(&pair.0);
        consume(&pair.1[0]);
        if total > 100 {
            return total;
        } else {
            consume(&total);
        }
        match maybe {
            Some(extra) => consume(extra),
            _ => continue,
        }
        let bumped = maybe.as_ref().map(|value| *value + 1);
        consume(&bumped.unwrap_or(0));
    }
    total
}

fn subpattern(first: Vec<(i32, i32)>, second: Vec<(i32, i32)>) {
    // Trigger when bindings with subpatterns and wildcards correspond.
    for whole @ (left, _) in first {
        show(whole);
        consume(&left);
    }
    for whole @ (left, _) in second {
        show(whole);
        consume(&left);
    }
}

fn formatted(first: &[i32], second: &[i32]) {
    // Trigger when the shared body formats the binding.
    for value in first {
        println!("{value}");
    }
    for value in second {
        println!("{value}");
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

fn unrelated_binding(first: &[i32], second: &[i32], outer: &i32) {
    // Keep quiet when one body uses its binding and the other uses an outer local.
    for value in first {
        consume(value);
    }
    for _value in second {
        consume(outer);
    }
}

fn swapped_positions(first: Vec<(i32, i32)>, second: Vec<(i32, i32)>) {
    // Keep quiet when the bindings occupy different pattern positions.
    for (value, _) in first {
        consume(&value);
    }
    for (_, value) in second {
        consume(&value);
    }
}

fn different_subpattern(first: Vec<(i32, i32)>, second: Vec<(i32, i32)>) {
    // Keep quiet when only one binding has a subpattern.
    for whole @ (_, _) in first {
        show(whole);
    }
    for whole in second {
        show(whole);
    }
}

fn different_item_types(first: &[i32], second: &[i64]) {
    // Keep quiet when the loops yield different item types.
    for value in first {
        show(value);
    }
    for value in second {
        show(value);
    }
}

fn different_closures(first: &[i32], second: &[i32]) {
    // Keep quiet when the closure bodies differ.
    for value in first {
        show(Some(value).map(|inner| *inner + 1));
    }
    for value in second {
        show(Some(value).map(|inner| *inner + 2));
    }
}

fn statement_kinds(first: &[i32], second: &[i32]) {
    // Keep quiet when the statements have different kinds.
    for value in first {
        let _copy = value;
    }
    for value in second {
        consume(value);
    }
}

fn literal_types(first: &[i32], second: &[i32]) {
    // Keep quiet when calls instantiate a generic function with different types.
    for _value in first {
        show(1_i32);
    }
    for _value in second {
        show(1_u8);
    }
}

fn breaking(first: &[i32], second: &[i32]) {
    // Keep quiet when `break` would leave both chained sequences.
    for value in first {
        if *value > 0 {
            break;
        }
    }
    for value in second {
        if *value > 0 {
            break;
        }
    }
}

fn missing_else(first: &[i32], second: &[i32]) {
    // Keep quiet when only one conditional has an `else` branch.
    for value in first {
        if *value > 0 {
            consume(value);
        }
    }
    for value in second {
        if *value > 0 {
            consume(value);
        } else {
            consume(value);
        }
    }
}

fn method_sources(first: &[i32], second: &[i32]) {
    // Keep quiet when a source is a call rather than a local binding.
    for value in first.iter() {
        consume(value);
    }
    for value in second.iter() {
        consume(value);
    }
}

#[allow(unused_labels)]
fn labeled(first: &[i32], second: &[i32]) {
    // Keep quiet for labeled blocks and other unsupported expression forms.
    for value in first {
        'body: {
            consume(value);
        }
    }
    for value in second {
        'body: {
            consume(value);
        }
    }
}

macro_rules! twice {
    ($first:expr, $second:expr) => {
        for value in $first {
            consume(value);
        }
        for value in $second {
            consume(value);
        }
    };
}

fn generated(first: &[i32], second: &[i32]) {
    // Keep quiet for loops produced by a macro.
    twice!(first, second);
}

fn main() {
    chained(&[1], &[2]);
    renamed(&[1], &[2]);
    tail_loop(vec![1], vec![2]);
    mutable_borrow(vec![1], vec![2]);
    let _ = rich(&[(1, Some(2))], &[(3, None)]);
    subpattern(vec![(1, 2)], vec![(3, 4)]);
    formatted(&[1], &[2]);
    different(&[1], &[2]);
    unrelated_binding(&[1], &[2], &3);
    swapped_positions(vec![(1, 2)], vec![(3, 4)]);
    different_subpattern(vec![(1, 2)], vec![(3, 4)]);
    different_item_types(&[1], &[2]);
    different_closures(&[1], &[2]);
    statement_kinds(&[1], &[2]);
    literal_types(&[1], &[2]);
    breaking(&[1], &[2]);
    missing_else(&[1], &[2]);
    method_sources(&[1], &[2]);
    labeled(&[1], &[2]);
    generated(&[1], &[2]);
}
