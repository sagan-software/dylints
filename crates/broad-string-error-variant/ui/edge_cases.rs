#![allow(dead_code)]

enum NetworkError {
    Timeout { details: String },
    Message(&'static str),
    Source(std::io::Error),
    Unknown,
}

enum ValidationErrors {
    Rejected { error: &'static str },
    Context { field: String },
}

enum DomainEvent {
    Message(String),
}

type Text = String;
type Borrowed<'a> = &'a str;

enum AliasError<'a> {
    Aliased { reason: Text },
    Details(Borrowed<'a>),
}

mod lookalike {
    pub struct String;

    pub enum LocalError {
        Message(String),
    }
}

macro_rules! generated_error {
    () => {
        enum GeneratedError {
            Message(String),
        }
    };
}

generated_error!();

fn main() {}
