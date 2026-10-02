#![allow(dead_code)]

struct RawToken;
struct Token;

fn from_raw_token(raw: RawToken) -> Token {
    let _ = raw;
    Token
}

fn token_from_parts(left: RawToken, right: RawToken) -> Token {
    let _ = (left, right);
    Token
}

fn make_policy_token(raw: RawToken) -> Token {
    let _ = raw;
    Token
}

fn main() {}
