#![allow(dead_code)]

struct Token {
    value: String,
    scope: String,
}

impl Token {
    pub fn new(value: String, scope: String) -> Self {
        Self { value, scope }
    }

    pub fn with_default_scope(value: String) -> Self {
        Self {
            value,
            scope: "default".to_owned(),
        }
    }
}

struct TupleToken(String);

impl TupleToken {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

fn main() {}
