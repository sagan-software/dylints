#![allow(dead_code)]

struct AccountId(String);

impl AccountId {
    fn format(&self) -> String {
        self.0.clone()
    }

    fn render(&self) -> String {
        self.0.clone()
    }

    fn format_for_log(&self, prefix: &str) -> String {
        format!("{prefix}:{}", self.0)
    }
}

fn main() {}
