#![allow(dead_code)]

enum NetworkError {
    Timeout { details: String },
    Message(&'static str),
    Source(std::io::Error),
}

enum ValidationErrors {
    Rejected { error: &'static str },
    Context { field: String },
}

enum DomainEvent {
    Message(String),
}

fn main() {}
