fn parse(value: &str) -> Option<i32> {
    value.parse().ok()
}

fn consume(_value: i32) {}

fn consume_option(_value: Option<i32>) {}

fn conditional(values: &[&str]) {
    for value in values {
        if let Some(mapped) = parse(value) {
            consume(mapped);
        }
    }
}

fn keyword_like_name(returned: &[&str]) {
    // A name that contains a keyword is not control flow.
    for value in returned {
        if let Some(mapped) = parse(value) {
            consume_option(value.parse::<i32>().ok().map(|number| number + mapped));
        }
    }
}

fn first_character(values: &[&str]) {
    // A trait method that returns `Option` also maps conditionally.
    for value in values {
        if let Some(character) = value.chars().next() {
            consume(character as i32);
        }
    }
}

fn two_actions(values: &[&str]) {
    // Keep quiet when the branch holds more than one action.
    for value in values {
        if let Some(mapped) = parse(value) {
            consume(mapped);
            consume(mapped);
        }
    }
}

fn compound_assignment(values: &[&str], total: &mut i32) {
    for value in values {
        if let Some(mapped) = parse(value) {
            *total += mapped;
        }
    }
}

fn boolean_condition(values: &[&str]) {
    // A plain predicate belongs to `manual_filter_for_each_loop`.
    for value in values {
        if value.is_empty() {
            consume(0);
        }
    }
}

fn unconditional(values: &[&str]) {
    for value in values {
        consume_option(parse(value));
    }
}

fn result_conversion(values: &[&str]) {
    // `if let Some(..) = result.ok()` is better written as `if let Ok(..)`.
    for value in values {
        if let Some(mapped) = value.parse::<i32>().ok() {
            consume(mapped);
        }
    }
}

fn result_pattern(values: &[&str]) {
    // A `Result` pattern is not an `Option` mapping.
    for value in values {
        if let Ok(mapped) = value.parse::<i32>() {
            consume(mapped);
        }
    }
}

fn none_pattern(values: &[Option<i32>]) {
    for value in values {
        if let None = value {
            consume(0);
        }
    }
}

fn early_exit(values: &[&str]) -> Option<()> {
    // `?` exits the function from inside the loop.
    for value in values {
        if let Some(mapped) = parse(value) {
            consume(mapped.checked_add(1)?);
        }
    }
    Some(())
}

macro_rules! generated {
    ($values:expr) => {
        for value in $values {
            if let Some(mapped) = parse(value) {
                consume(mapped);
            }
        }
    };
}

fn macro_loop(values: &[&str]) {
    generated!(values);
}

fn main() {
    conditional(&["1"]);
    result_conversion(&["1"]);
}

fn non_exhaustive_payload(values: &[Option<i32>]) {
    // Keep quiet because `filter_map` would run the action for every `Some` value.
    for value in values {
        if let Some(0) = value {
            consume(0);
        }
    }
}
