// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use std::fmt::{Debug, Formatter, Result};

struct Point(u64, u64);

impl Debug for Point {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple("Point")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

struct Redacted {
    secret: String,
}

impl Debug for Redacted {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Redacted")
            .field("secret", &"<redacted>")
            .finish()
    }
}

fn main() {}
