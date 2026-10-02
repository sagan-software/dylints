#![allow(dead_code)]

struct Token(String);

impl Token {
    fn borrow_token(&self) -> &String {
        &self.0
    }

    fn token(&self) -> &str {
        &self.0
    }

    fn borrow_for_scope(&self, _scope: &str) -> &String {
        &self.0
    }
}

fn main() {}
