use std::fmt::{Debug, Formatter, Result};

struct User {
    id: u64,
    name: String,
}

impl Debug for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

struct Point(i32, i32);

impl std::fmt::Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Point")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

struct ApiSecret {
    token: String,
}

impl Debug for ApiSecret {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ApiSecret")
            .field("token", &"<redacted>")
            .finish()
    }
}

struct Conditional {
    value: u64,
}

impl Debug for Conditional {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if f.alternate() {
            f.debug_struct("Conditional")
                .field("value", &self.value)
                .finish()
        } else {
            f.debug_tuple("Conditional").field(&self.value).finish()
        }
    }
}

fn main() {}
