#![allow(dead_code)]

struct Token;
struct Error;

fn parse_token(raw: &str) -> Result<Token, Error> {
    let _ = raw;
    Ok(Token)
}

fn read_token(raw: &str) -> Result<Token, Error> {
    let _ = raw;
    Ok(Token)
}

fn parse_with_policy(raw: &str, policy: &str) -> Result<Token, Error> {
    let _ = (raw, policy);
    Ok(Token)
}

fn main() {}
