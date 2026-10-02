#![allow(dead_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
struct TokenError {
    token_id: u64,
}

impl Display for TokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad token {}", self.token_id)
    }
}

impl Error for TokenError {}

#[derive(Debug)]
struct SourceError;

impl Display for SourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "source error")
    }
}

impl Error for SourceError {}

fn main() {}
