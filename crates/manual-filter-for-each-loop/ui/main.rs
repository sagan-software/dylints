// compile-flags: --edition 2024

fn consume(_value: i32) {}

fn filtered(values: &[i32]) {
    for value in values {
        if *value > 0 {
            consume(*value);
        }
    }
}

fn keyword_like_name(returned: &[i32], total: &mut i32) {
    // A name that contains a keyword is not control flow.
    for value in returned {
        if *value > 0 && *value < 10 {
            *total = *value;
        }
    }
}

fn closure_return(values: &[i32]) {
    // A `return` inside a closure does not leave the loop.
    for value in values {
        if *value > 0 {
            consume((|| return *value)());
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

fn if_let(values: &[Option<i32>]) {
    // `if let` belongs to `manual_filter_map_for_each_loop`.
    for value in values {
        if let Some(number) = value {
            consume(*number);
        }
    }
}

fn let_chain(values: &[Option<i32>]) {
    // A let chain has no direct `filter` equivalent.
    for value in values {
        if value.is_some()
            && let Some(number) = value
        {
            consume(*number);
        }
    }
}

fn early_return(values: &[i32]) {
    for value in values {
        if *value > 0 {
            return consume(*value);
        }
    }
}

fn question_mark(values: &[Option<i32>]) -> Option<()> {
    for value in values {
        if value.is_some() {
            consume((*value)?);
        }
    }
    Some(())
}

fn block_action(values: &[i32]) {
    // Keep quiet when the branch holds more than one action.
    for value in values {
        if *value > 0 {
            consume(*value);
            consume(*value);
        }
    }
}

fn compound_assignment(values: &[i32], total: &mut i32) {
    for value in values {
        if *value > 0 {
            *total += *value;
        }
    }
}

macro_rules! generated {
    ($values:expr) => {
        for value in $values {
            if *value > 0 {
                consume(*value);
            }
        }
    };
}

fn macro_loop(values: &[i32]) {
    generated!(values);
}

fn main() {
    filtered(&[1, -1]);
    with_else(&[1, -1]);
}
