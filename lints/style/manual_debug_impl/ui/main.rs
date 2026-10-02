// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

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

struct Empty {}

impl Debug for Empty {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Empty").finish()
    }
}

struct Reordered {
    first: u8,
    second: u8,
}

impl Debug for Reordered {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Reordered")
            .field("second", &self.second)
            .field("first", &self.first)
            .finish()
    }
}

struct Renamed {
    value: u8,
}

impl Debug for Renamed {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Other").field("value", &self.value).finish()
    }
}

struct Partial {
    shown: u8,
    hidden: u8,
}

impl Debug for Partial {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Partial")
            .field("shown", &self.shown)
            .finish_non_exhaustive()
    }
}

struct Documented {
    value: u8,
}

/// The documentation stays with this impl, so the lint offers no fix.
impl Debug for Documented {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Documented").field("value", &self.value).finish()
    }
}

struct Generic<T>(T);

impl<T: Debug> Debug for Generic<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple("Generic").field(&self.0).finish()
    }
}

enum Kind {
    One,
}

impl Debug for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("One")
    }
}

fn main() {}
