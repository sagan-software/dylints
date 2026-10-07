use std::{
    collections::{HashSet, VecDeque},
    error::Error,
    io,
    num::ParseIntError,
};

fn parse_all(values: &[&str]) -> Result<Vec<i32>, ParseIntError> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse()?);
    }
    Ok(output)
}

fn question_mark_in_text(values: &[&str]) -> Result<Vec<i32>, String> {
    // A `?` inside a string literal is not an operator.
    let mut output = Vec::default();
    for value in values {
        output.push(value.parse::<i32>().map_err(|_| String::from("bad ?"))?);
    }
    Ok(output)
}

fn parse_set(values: &[&str]) -> Result<HashSet<i32>, ParseIntError> {
    let mut output = HashSet::new();
    for value in values {
        output.insert(value.parse()?);
    }
    Ok(output)
}

fn parse_deque(values: &[&str]) -> Option<VecDeque<i32>> {
    let mut output: VecDeque<i32> = Default::default();
    for value in values {
        output.push_back(value.parse().ok()?);
    }
    Some(output)
}

fn converted_error(values: &[&str]) -> io::Result<Vec<i32>> {
    // Keep quiet when the loop body performs another action.
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse().map_err(io::Error::other)?);
        println!("parsed {value}");
    }
    Ok(output)
}

fn from_conversion(values: &[&str]) -> Result<Vec<i32>, Box<dyn Error>> {
    // `?` converts the error through `From`, which `collect` does not do.
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse::<i32>()?);
    }
    Ok(output)
}

fn parse_optional(values: &[&str]) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse().ok()?);
    }
    Some(output)
}

fn two_operators(values: &[&str]) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse::<i32>().ok()?.checked_add(1)?);
    }
    Some(output)
}

fn early_return(values: &[&str]) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    for value in values {
        output.push(if value.is_empty() {
            return None;
        } else {
            value.parse().ok()?
        });
    }
    Some(output)
}

fn prefilled(values: &[&str]) -> Option<Vec<i32>> {
    // A prefilled collection keeps its first items.
    let mut output = vec![0];
    for value in values {
        output.push(value.parse().ok()?);
    }
    Some(output)
}

fn other_tail(values: &[&str], other: Vec<i32>) -> Option<Vec<i32>> {
    let mut output: Vec<i32> = Vec::new();
    for value in values {
        output.push(value.parse().ok()?);
    }
    let _ = output;
    Some(other)
}

fn other_target(values: &[&str], other: &mut Vec<i32>) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    for value in values {
        other.push(value.parse().ok()?);
    }
    Some(output)
}

fn not_insertion(values: &[&str]) -> Option<Vec<i32>> {
    let mut output: Vec<i32> = Vec::new();
    for value in values {
        output.truncate(value.parse().ok()?);
    }
    Some(output)
}

fn not_loop(values: &[&str]) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    output.push(values.first()?.parse().ok()?);
    Some(output)
}

fn assignment(values: &[&str]) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    for value in values {
        output = vec![value.parse().ok()?];
    }
    Some(output)
}

fn scalar_sum(values: &[&str]) -> Option<i32> {
    // A scalar accumulator is not a collection.
    let mut total = 0;
    for value in values {
        total += value.parse::<i32>().ok()?;
    }
    Some(total)
}

fn plain_tail(values: &[&str]) -> usize {
    let count = values.len();
    count
}

fn binding_tail(values: &[&str]) -> Option<usize> {
    let count = Some(values.len());
    count
}

fn function_tail(values: &[&str]) -> Option<usize> {
    let count = values.len();
    std::convert::identity(Some(count))
}

fn closure_tail(values: &[&str]) -> Option<usize> {
    let count = values.len();
    (|count| Some(count))(count)
}

fn main() {
    let _ = parse_all(&["1"]);
    let _ = converted_error(&["1"]);
    let _ = parse_optional(&["1"]);
    let _ = trait_constructor(&["1"]);
}

struct ForeignFactory;

impl ForeignFactory {
    fn new() -> Vec<i32> {
        vec![99]
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

fn foreign_constructor(values: &[&str]) -> Option<Vec<i32>> {
    // Keep quiet because `collect` would drop the existing item from this collection.
    let mut output: Vec<i32> = ForeignFactory::new();
    for value in values {
        output.push(value.parse::<i32>().ok()?);
    }
    Some(output)
}

fn trait_constructor(values: &[&str]) -> Option<Vec<i32>> {
    // Keep quiet because the trait constructor returns a nonempty collection.
    let mut output: Vec<i32> = <Vec<i32> as Factory>::new();
    for value in values {
        output.push(value.parse::<i32>().ok()?);
    }
    Some(output)
}
