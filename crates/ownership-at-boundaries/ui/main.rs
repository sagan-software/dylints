type ErrorText = String;
type ErrorLines = std::vec::Vec<String>;

struct TextWrapper(String);

pub fn has_prefix(value: String, prefix: String) -> bool {
    value.starts_with(&prefix)
}

pub fn count_errors(lines: Vec<String>) -> usize {
    lines.iter().filter(|line| line.contains("ERROR")).count()
}

pub fn aliased(value: ErrorText, lines: ErrorLines) -> usize {
    value.len() + lines.len()
}

pub fn fully_qualified(
    value: std::string::String,
    lines: std::vec::Vec<std::string::String>,
) -> usize {
    value.len() + lines.len()
}

fn private_helper(value: String) -> usize {
    value.len()
}

fn borrowed(value: &str, lines: &[String]) -> bool {
    value.is_empty() || lines.is_empty()
}

fn wrapper(value: TextWrapper) -> usize {
    value.0.len()
}

mod domain_types {
    pub struct String;
    pub struct Vec<T>(pub T);

    pub fn custom_types(value: String, lines: Vec<String>) {
        let _ = (value, lines);
    }
}

fn main() {}
