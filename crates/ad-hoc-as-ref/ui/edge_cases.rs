#![allow(dead_code)]

struct Header(String);

impl Header {
    fn get_header(&self) -> &String {
        &self.0
    }

    fn as_header(&self) -> &str {
        &self.0
    }

    fn header_for_scope(&self, _scope: &str) -> &String {
        &self.0
    }
}

fn main() {}
