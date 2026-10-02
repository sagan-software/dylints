#![allow(dead_code)]

struct Error;

struct Payload {
    raw: String,
}

impl Payload {
    fn as_str(&self) -> &str {
        &self.raw
    }

    fn as_parsed(&self) -> Result<u64, Error> {
        self.raw.parse::<u64>().map_err(|_| Error)
    }

    fn as_owned_string(&self) -> String {
        self.raw.clone()
    }
}

fn main() {}
