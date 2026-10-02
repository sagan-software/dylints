use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
struct MissingUser {
    user_id: u64,
}

impl Display for MissingUser {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing user {}", self.user_id)
    }
}

impl Error for MissingUser {}

#[derive(Debug)]
struct Qualified {
    path: String,
}

impl std::fmt::Display for Qualified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid path: {}", self.path)
    }
}

impl std::error::Error for Qualified {}

#[derive(Debug)]
struct Wrapped {
    source: std::io::Error,
}

impl Display for Wrapped {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "io failure")
    }
}

impl Error for Wrapped {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
struct Redacted {
    token: String,
}

impl Display for Redacted {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "token {}", self.token)
        } else {
            write!(f, "token <redacted>")
        }
    }
}

impl Error for Redacted {}

#[derive(Debug)]
struct DisplayOnly;

impl Display for DisplayOnly {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "display only")
    }
}

fn main() {}
