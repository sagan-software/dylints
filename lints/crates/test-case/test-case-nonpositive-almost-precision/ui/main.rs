// compile-flags: --test

use test_case::test_case;

#[test_case(1.0_f64 => is almost 1.0 precision 0.0 ; "zero precision")]
fn zero(value: f64) -> f64 {
    value
}

#[test_case(1.0_f64 => is almost 1.0 precision -0.1 ; "negative precision")]
fn negative(value: f64) -> f64 {
    value
}

#[test_case(1.0_f64 => is almost 1.0 precision 0.01 ; "positive precision")]
fn positive(value: f64) -> f64 {
    value
}

fn main() {}
