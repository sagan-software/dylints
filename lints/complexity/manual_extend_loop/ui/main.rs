use std::collections::VecDeque;

fn extend_vec(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        output.push(value);
    }
}

fn extend_deque(output: &mut VecDeque<i32>, values: Vec<i32>) {
    for value in values {
        output.push_back(value * 2);
    }
}

struct Custom(Vec<i32>);

impl Custom {
    fn push(&mut self, value: i32) {
        self.0.push(value);
    }
}

fn custom(output: &mut Custom, values: Vec<i32>) {
    for value in values {
        output.push(value);
    }
}

fn main() {
    extend_vec(&mut Vec::new(), vec![1]);
    extend_deque(&mut VecDeque::new(), vec![1]);
    custom(&mut Custom(Vec::new()), vec![1]);
}
