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

mod first {
    #[derive(Debug)]
    pub struct Duplicate;

    impl std::fmt::Display for Duplicate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "first duplicate")
        }
    }
}

mod second {
    #[derive(Debug)]
    pub struct Duplicate;

    impl std::error::Error for Duplicate {}

    impl std::fmt::Display for Duplicate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("second duplicate")
        }
    }
}

mod local_traits {
    pub trait Display {
        fn show(&self) -> String;
    }

    pub trait Error {}

    #[derive(Debug)]
    pub struct LocalTraits;

    impl Display for LocalTraits {
        fn show(&self) -> String {
            String::new()
        }
    }

    impl Error for LocalTraits {}
}

trait PrimitiveMarker {}

impl PrimitiveMarker for u8 {}

#[derive(Debug)]
struct ErrorFirst;

impl Error for ErrorFirst {}

impl Display for ErrorFirst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "error first")
    }
}

#[derive(Debug)]
struct GenericError<T>(T);

impl<T> Display for GenericError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic")
    }
}

impl<T: std::fmt::Debug> Error for GenericError<T> {}

#[derive(Debug)]
struct TwoStatements;

impl Display for TwoStatements {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "two ")?;
        write!(f, "statements")
    }
}

impl Error for TwoStatements {}

fn main() {}
