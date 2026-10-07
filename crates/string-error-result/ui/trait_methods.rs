#![allow(dead_code)]

trait Reader {
    fn read(&self) -> Result<(), String>;
}

trait Flusher {
    fn flush(&self);
}

trait TypedReader {
    fn read(&self) -> Result<(), std::io::Error>;
}

trait UnresolvedReader {
    type Error;

    fn read(&self) -> Result<(), Self::Error>;
}

trait BoxedErrorReader {
    fn read(&self) -> Result<(), Box<dyn std::error::Error>>;
}

mod domain_result {
    pub struct Result<T, E>(pub T, pub E);

    pub trait Reader {
        fn read(&self) -> Result<(), std::string::String>;
    }
}

fn main() {}
