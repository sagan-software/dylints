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

fn main() {}
