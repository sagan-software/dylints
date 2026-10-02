// compile-flags: --edition 2024
#![allow(dead_code)]

use std::str::FromStr;

enum State {
    Queued,
    Running,
    Complete,
}

impl FromStr for State {
    type Err = String;

    // Parsing a closed vocabulary from text is the intended boundary.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "complete" => Ok(Self::Complete),
            other => Err(other.to_owned()),
        }
    }
}

impl TryFrom<&str> for State {
    type Error = ();

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let parse = |text: &str| match text {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "complete" => Ok(Self::Complete),
            _ => Err(()),
        };
        parse(input)
    }
}

trait Store {
    fn load(&self, account_id: u64) -> Option<u64>;
}

struct Memory;

// The trait fixes this signature, so only the trait declaration is reported.
impl Store for Memory {
    fn load(&self, account_id: u64) -> Option<u64> {
        Some(account_id)
    }
}

fn one_line_arms(state: String) -> u8 {
    match state.as_str() { "queued" => 0, "running" | "paused" => 1, "complete" => 2, other => other.len() as u8 }
}

fn guarded_fallback(state: &str) -> u8 {
    match state {
        "queued" => 0,
        "running" => 1,
        "complete" => 2,
        other if other.is_empty() => 3,
        _ => 4,
    }
}

fn two_literals(state: &str) -> u8 {
    match state {
        "queued" => 0,
        "running" => 1,
        _ => 2,
    }
}

fn no_fallback(flag: bool) -> u8 {
    match flag {
        true => 1,
        false => 0,
    }
}

fn integer_match(code: u8) -> u8 {
    match code {
        1 => 0,
        2 => 1,
        3 => 2,
        _ => 3,
    }
}

fn closures() {
    let _ = |account_id: u64| account_id;
}

fn destructured((account_id, _): (u64, u8)) -> u64 {
    account_id
}

fn main() {}
