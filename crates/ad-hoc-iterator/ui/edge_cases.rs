#![allow(dead_code)]

struct Row;

struct Cursor(Vec<Row>);

impl Cursor {
    fn next_item(&mut self) -> Option<Row> {
        self.0.pop()
    }

    fn next_batch(&mut self) -> Vec<Row> {
        std::mem::take(&mut self.0)
    }

    fn peek_item(&self) -> Option<&Row> {
        self.0.last()
    }
}

fn main() {}
