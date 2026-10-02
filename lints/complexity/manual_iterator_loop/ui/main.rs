fn collect_doubled(values: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    for value in values {
        out.push(value * 2);
    }
    out
}

fn count_even(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value % 2 == 0 {
            count += 1;
        }
    }
    count
}

fn any_even(values: &[i32]) -> bool {
    let mut found = false;
    for value in values {
        if *value % 2 == 0 {
            found = true;
        }
    }
    found
}

fn all_even(values: &[i32]) -> bool {
    let mut all_ok = true;
    for value in values {
        if *value % 2 != 0 {
            all_ok = false;
        }
    }
    all_ok
}

fn collect_alias_positive(values: &[i32]) -> Vec<i32> {
    type Output<T> = Vec<T>;

    let mut out: Output<i32> = Output::new();
    for value in values {
        out.push(*value);
    }
    out
}

fn collect_qualified_positive(values: &[i32]) -> Vec<i32> {
    let mut out = std::vec::Vec::new();
    for value in values {
        out.push(*value);
    }
    out
}

fn collect_renamed_import_positive(values: &[i32]) -> Vec<i32> {
    use std::vec::Vec as Bag;

    let mut out: Bag<i32> = Bag::new();
    for value in values {
        out.push(*value);
    }
    out
}

struct VecLike<T> {
    values: Vec<T>,
}

impl<T> VecLike<T> {
    fn new() -> Self {
        Self { values: Vec::new() }
    }

    fn push(&mut self, value: T) {
        self.values.push(value);
    }
}

fn custom_push_lookalike(values: &[i32]) -> VecLike<i32> {
    let mut out = VecLike::new();
    for value in values {
        out.push(*value);
    }
    out
}

fn push_to_different_accumulator(values: &[i32]) -> Vec<i32> {
    let mut other = Vec::new();
    let mut out = Vec::new();
    for value in values {
        other.push(*value);
    }
    out
}

fn count_different_accumulator(values: &[i32]) -> usize {
    let mut other = 0usize;
    let mut count = 0usize;
    for value in values {
        if *value > 0 {
            other += 1;
        }
    }
    count
}

fn bool_assignment_does_not_match(values: &[i32]) -> bool {
    let mut found = false;
    for value in values {
        if *value > 0 {
            found = false;
        }
    }
    found
}

fn complex_push_body(values: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    for value in values {
        println!("{value}");
        out.push(*value);
    }
    out
}

fn separated_initializer(values: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    let _capacity = values.len();
    for value in values {
        out.push(*value);
    }
    out
}

fn complex_count_body(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            println!("{value}");
            count += 1;
        }
    }
    count
}

fn main() {}

// An unconditional assignment cannot preserve the suggested `any` semantics.
fn unconditional_bool_assignment(values: &[i32]) -> bool {
    let mut found = false;
    for value in values {
        found = *value > 0;
    }
    found
}

// An unrelated call in the conditional body is not an accumulator increment.
fn call_instead_of_increment(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value > 0 {
            std::hint::black_box(value);
        }
    }
    count
}
