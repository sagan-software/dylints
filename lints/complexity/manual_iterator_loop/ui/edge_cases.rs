#![allow(dead_code)]

fn collect_with_filter(values: &[i32]) -> Vec<i32> {
    let mut output = Vec::new();
    for value in values {
        if *value > 0 {
            output.push(value * 2);
        }
    }
    output
}

fn count_nested_condition(values: &[i32]) -> usize {
    let mut matches = 0;
    for value in values {
        if *value > 0 && *value % 2 == 0 {
            matches += 1;
        }
    }
    matches
}

fn any_with_break(values: &[i32]) -> bool {
    let mut found = false;
    for value in values {
        if *value == 42 {
            found = true;
            break;
        }
    }
    found
}

fn push_to_existing(values: &[i32], output: &mut Vec<i32>) {
    for value in values {
        output.push(*value);
    }
}

fn push_with_try(values: &[&str]) -> Result<Vec<i32>, std::num::ParseIntError> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse::<i32>()?);
    }
    Ok(output)
}

fn count_with_match_return(values: &[Option<i32>]) -> usize {
    let mut count = 0;
    for value in values {
        if match value {
            Some(inner) => *inner > 0,
            None => return 0,
        } {
            count += 1;
        }
    }
    count
}

fn any_with_closure_return(values: &[Vec<i32>]) -> bool {
    let mut found = false;
    for value in values {
        if value.iter().any(|inner| {
            if *inner < 0 {
                return false;
            }
            *inner > 10
        }) {
            found = true;
        }
    }
    found
}

fn push_with_block_binding(values: &[i32]) -> Vec<i32> {
    let mut output = Vec::new();
    for value in values {
        output.push({
            let doubled = value * 2;
            doubled + 1
        });
    }
    output
}

macro_rules! generated_accumulator {
    ($values:expr) => {{
        let mut output = Vec::new();
        for value in $values {
            output.push(*value);
        }
        output
    }};
}

fn macro_generated(values: &[i32]) -> Vec<i32> {
    generated_accumulator!(values)
}

// The predicate must not depend on the accumulator that changes during iteration.
fn count_depends_on_accumulator(values: &[i32]) -> usize {
    let mut count = 0;
    for _value in values {
        if count < 2 {
            count += 1;
        }
    }
    count
}

fn count_captures_accumulator(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if (|| count < *value as usize)() {
            count += 1;
        }
    }
    count
}

fn any_depends_on_accumulator(values: &[i32]) -> bool {
    let mut found = false;
    for value in values {
        if !found && *value > 0 {
            found = true;
        }
    }
    found
}

fn all_depends_on_accumulator(values: &[i32]) -> bool {
    let mut all = true;
    for value in values {
        if all && *value < 0 {
            all = false;
        }
    }
    all
}

fn mapped_value_depends_on_accumulator(values: &[i32]) -> Vec<usize> {
    let mut output = Vec::new();
    for _value in values {
        output.push(output.len());
    }
    output
}

fn main() {}

fn count_assignment(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            count = count + 1;
        }
    }
    count
}

fn count_reverse_addition(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            count = 1 + count;
        }
    }
    count
}

fn count_larger_increment(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            count = count + 2;
        }
    }
    count
}

fn count_replacement(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            count = 1;
        }
    }
    count
}

fn count_without_condition(values: &[i32]) -> usize {
    let mut count = 0;
    for _value in values {
        count += 1;
    }
    count
}

fn count_condition_without_increment(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            let _ = value;
        }
    }
    count += 0;
    count
}

fn boolean_condition_without_assignment(values: &[i32]) -> bool {
    let mut found = false;
    for value in values {
        if *value > 0 {
            ignore_value(value);
        }
    }
    found |= false;
    found
}

fn multiple_pushes(values: &[i32]) -> Vec<i32> {
    let mut output = Vec::new();
    for value in values {
        output.push(*value);
        output.push(*value);
    }
    output
}

fn push_function(output: &mut Vec<i32>, value: i32) {
    output.push(value);
}

fn non_method_push(values: &[i32]) -> Vec<i32> {
    let mut output = Vec::new();
    for value in values {
        push_function(&mut output, *value);
    }
    output
}

fn closure_constructor(values: &[i32]) -> Vec<i32> {
    let mut output = (|| Vec::new())();
    for value in values {
        output.push(*value);
    }
    output
}

fn pointer_constructor(values: &[i32]) -> Vec<i32> {
    let create = Vec::new;
    let mut output = create();
    for value in values {
        output.push(*value);
    }
    output
}

fn shadowed_accumulator(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if (|| {
            let count = 2;
            *value < count
        })() {
            count += 1;
        }
    }
    count
}

fn ignore_value(value: &i32) {
    let _ = value;
}
