// compile-flags: --edition 2024

#![allow(dead_code)]

struct CohesiveState {
    input: usize,
    output: usize,
    attempts: usize,
    limit: usize,
}

impl CohesiveState {
    fn read_input(&self) -> usize {
        self.input
    }

    fn write_input(&mut self, value: usize) {
        self.input = value;
        self.attempts += 1;
    }

    fn read_output(&self) -> usize {
        self.output
    }

    fn write_output(&mut self, value: usize) {
        self.output = value;
        self.attempts += 1;
    }

    fn can_retry(&self) -> bool {
        self.attempts < self.limit
    }

    fn reset(&mut self) {
        self.input = 0;
        self.output = 0;
        self.attempts = 0;
    }
}

struct BuilderLike {
    first: usize,
    second: usize,
    third: usize,
    fourth: usize,
}

impl BuilderLike {
    fn first(&self) -> usize {
        self.first
    }
    fn set_first(&mut self, value: usize) {
        self.first = value;
    }
    fn second(&self) -> usize {
        self.second
    }
    fn set_second(&mut self, value: usize) {
        self.second = value;
    }
    fn third(&self) -> usize {
        self.third
    }
    fn set_third(&mut self, value: usize) {
        self.third = value;
    }
    fn fourth(&self) -> usize {
        self.fourth
    }
    fn set_fourth(&mut self, value: usize) {
        self.fourth = value;
    }
}

fn main() {}
