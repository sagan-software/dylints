#[cfg(feature = "abc")]
fn first() {}

#[cfg(feature = "abc")]
fn second() {}

#[cfg(feature = "xyz")]
fn boundary_one() {}

#[cfg(feature = "xyz")]
fn boundary_two() {}

#[cfg(feature = "xyz")]
fn boundary_three() {}

mod nested {
    include!("nested.in");
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "abc")]
    fn test_gate_one() {}

    #[cfg(feature = "abc")]
    fn test_gate_two() {}
}

#[cfg(feature = "abc")]
#[test]
fn test_function_gate() {}

fn main() {}
