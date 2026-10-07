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

struct BridgedByMacroArgument {
    left_a: usize,
    left_b: usize,
    right_a: usize,
    right_b: usize,
}

impl BridgedByMacroArgument {
    fn read_left(&self) -> usize {
        self.left_a + self.left_b
    }

    fn write_left(&mut self, value: usize) {
        self.left_a = value;
        self.left_b = value;
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

    fn report(&self) {
        println!("{} {}", self.left_a, self.right_a);
    }
}

struct BridgedByClosure {
    left_a: usize,
    left_b: usize,
    right_a: usize,
    right_b: usize,
}

impl BridgedByClosure {
    fn read_left(&self) -> usize {
        self.left_a + self.left_b
    }

    fn write_left(&mut self, value: usize) {
        self.left_a = value;
        self.left_b = value;
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

    fn total(&self) -> usize {
        let sum = || self.left_a + self.right_a;
        sum()
    }
}

struct BridgedByMethodCall {
    left_a: usize,
    left_b: usize,
    right_a: usize,
    right_b: usize,
}

impl BridgedByMethodCall {
    fn read_left(&self) -> usize {
        self.left_a + self.left_b
    }

    fn write_left(&mut self, value: usize) {
        self.left_a = value;
        self.left_b = value;
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

    fn summary(&self) -> usize {
        self.left_a.max(1) + self.read_right() + self.helper()
    }

    fn helper(&self) -> usize {
        self.left_b.min(1)
    }
}

struct TooSmall {
    a: usize,
    b: usize,
}

impl TooSmall {
    fn a(&self) -> usize {
        self.a
    }

    fn b(&self) -> usize {
        self.b
    }

    fn delegates(&self) -> usize {
        self.a() + self.b()
    }
}

trait DefaultReceiver {
    fn provided(&self) -> usize {
        0
    }
}

trait Marker {}

impl dyn Marker {
    fn describe(&self) -> usize {
        0
    }
}

fn main() {}
