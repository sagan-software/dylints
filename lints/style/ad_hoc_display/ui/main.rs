#![allow(dead_code)]

use std::fmt::{Display as StdDisplay, Formatter, Result as FmtResult};

type DisplayText = String;
type QualifiedDisplayText = std::string::String;

struct UserId(String);

impl UserId {
    fn to_string(&self) -> String {
        self.0.clone()
    }

    fn display(&self) -> DisplayText {
        self.0.clone()
    }

    fn render(&self) -> QualifiedDisplayText {
        self.0.clone()
    }

    fn format(&self) -> usize {
        self.0.len()
    }

    fn format_with_prefix(&self, prefix: &str) -> String {
        format!("{prefix}{}", self.0)
    }
}

impl StdDisplay for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

struct CachedLabel {
    value: String,
}

impl CachedLabel {
    fn display(&mut self) -> String {
        self.value.clone()
    }
}

fn main() {}
