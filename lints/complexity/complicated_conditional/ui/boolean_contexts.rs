// compile-flags: --edition 2024
#![allow(dead_code, unused_variables, unused_assignments)]

fn contexts(values: &[String], other: &[String]) {
    let mut is_ready = values.iter().next().is_some() && other.iter().next().is_some();
    is_ready = values.iter().next().is_some() || other.iter().next().is_some();
    while values.iter().next().is_some() && other.iter().next().is_some() {
        break;
    }
    match values.len() {
        _ if values.iter().next().is_some() && other.iter().next().is_some() => (),
        _ => (),
    }
    let has_match = values.iter().any(|value| {
        let trimmed = value.trim();
        let lowered = trimmed.to_lowercase();
        lowered.starts_with('a') && lowered.ends_with('z')
    }) && !other.is_empty();
    let is_simple = !values.is_empty() && !other.is_empty();
    let is_named = is_ready && has_match && is_simple;
    let non_boolean = values.iter().map(|value| value.trim()).collect::<Vec<_>>();
}

fn main() {}

struct Chain;
impl Chain {
    fn step(&self) -> &Self {
        self
    }
    fn ready(&self) -> bool {
        true
    }
}

fn thresholds(chain: &Chain) {
    // Three calls score six; four calls reach the standalone threshold of eight.
    let below = chain.step().step().ready();
    let just_below = !chain.step().step().ready();
    let at = chain.step().step().step().ready();
    let above = !chain.step().step().step().ready();
    // Two four-point terms trigger; two two-point calls remain accepted.
    let combined = chain.step().ready() && chain.step().ready();
    let simple = chain.ready() && chain.ready();
    let combined_below = chain.step().ready() && !chain.ready();
    let one_complex_at = chain.step().step().ready() && chain.ready();
    let bool_comparison = chain.step().ready() == chain.step().ready();
    // Named results remain readable even with many logical operators.
    if below && at && combined && simple {}
    // Closure work in an intermediate adapter contributes to the receiver chain.
    let adapted = [1, 2]
        .iter()
        .filter(|value| {
            let first = **value + 1;
            let second = first * 2;
            second > 2
        })
        .next()
        .is_some();
    let standalone = [1, 2].iter().any(|value| {
        let first = *value + 1;
        let second = first * 2;
        second > 2
    });
}

fn additional_contexts(chain: &Chain, values: &[i32]) {
    let explicit: bool = chain.step().step().step().ready();
    let uninitialized: bool;
    uninitialized = chain.step().step().step().ready();
    let mut non_boolean = 1;
    non_boolean = 2 + 3;
    let simple_closure = values.iter().any(|value| *value > 0);
    let with_branch = values
        .iter()
        .any(|value| if *value > 1 { chain.ready() } else { false });
    let with_match = values.iter().any(|value| match *value {
        0 => chain.ready(),
        _ => false,
    });
    let with_loop = values.iter().any(|value| {
        loop {
            break *value > 0;
        }
    });
    let with_assignment = values.iter().any(|value| {
        let mut result = *value;
        result = result + 1;
        result > 0
    });
    let nested_item = values.iter().any(|value| {
        fn unrelated() -> bool {
            let first = String::new();
            first.trim().to_lowercase().starts_with('a')
        }
        *value > 0
    });
    let generated = cfg!(debug_assertions) && chain.step().step().step().ready();
    let macro_body = values.iter().any(|_| cfg!(debug_assertions));
    let long_closure = values.iter().any(|value| {
        let a = *value + 1;
        let b = a + 1;
        let c = b + 1;
        let d = c + 1;
        let e = d + 1;
        e > 0
    });
    let function_argument = accepts(|| {
        let a = chain.ready();
        let b = chain.step().ready();
        a && b
    });
}

fn accepts(predicate: impl Fn() -> bool) -> bool {
    predicate()
}

fn comparisons(value: &str, other: &str) {
    let elaborate = value.trim().to_lowercase().trim().len() > 0;
    let combined = value.trim().len() > 0 && other.trim().len() > 0;
    let simple = value.len() > 0 && other.len() > 0;
}

fn pattern_conditions(values: &[i32]) {
    if let Some(value) = values.first() {}
    while let Some(value) = values.first() {
        break;
    }
    let Some(value) = values.first() else {
        return;
    };
}
