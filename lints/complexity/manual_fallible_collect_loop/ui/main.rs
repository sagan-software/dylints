use std::{io, num::ParseIntError};

fn parse_all(values: &[&str]) -> Result<Vec<i32>, ParseIntError> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse()?);
    }
    Ok(output)
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

fn parse_optional(values: &[&str]) -> Option<Vec<i32>> {
    let mut output = Vec::new();
    for value in values {
        output.push(value.parse().ok()?);
    }
    Some(output)
}

fn main() {
    let _ = parse_all(&["1"]);
    let _ = converted_error(&["1"]);
    let _ = parse_optional(&["1"]);
}
