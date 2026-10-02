#![allow(dead_code)]

use std::fmt::{Display as StdDisplay, Formatter, Result as FmtResult};

type DisplayText = String;
type QualifiedDisplayText = std::string::String;

struct UserId(String);

impl UserId {
    const PREFIX: &'static str = "user";

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

    fn display_static() -> String {
        String::new()
    }
}

struct ShownId(String);

impl ShownId {
    fn render(&self) -> String {
        self.0.clone()
    }
}

impl StdDisplay for ShownId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

trait Renderer {
    fn render(&self) -> String;
}

impl Renderer for UserId {
    fn render(&self) -> String {
        self.0.clone()
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
