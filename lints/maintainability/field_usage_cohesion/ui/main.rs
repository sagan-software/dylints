// compile-flags: --edition 2024

#![allow(dead_code)]

struct SplitState {
    left_a: usize,
    left_b: usize,
    right_a: usize,
    right_b: usize,
}

impl SplitState {
    fn read_left(&self) -> usize {
        self.left_a + self.left_b
    }

    fn write_left(&mut self, value: usize) {
        self.left_a = value;
        self.left_b = value;
    }

    fn reset_left(&mut self) {
        self.left_a = 0;
        self.left_b = 0;
    }

    fn read_right(&self) -> usize {
        self.right_a + self.right_b
    }

    fn write_right(&mut self, value: usize) {
        self.right_a = value;
        self.right_b = value;
    }

    fn reset_right(&mut self) {
        self.right_a = 0;
        self.right_b = 0;
    }
}

fn main() {}
