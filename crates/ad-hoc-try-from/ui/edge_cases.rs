#![allow(dead_code)]

use std::convert::TryFrom;

struct RawToken;
struct Token;
struct Error;

type TokenResult = Result<Token, Error>;

mod local {
    use std::marker::PhantomData;

    pub struct Result<T, E>(pub PhantomData<(T, E)>);
}

fn try_token(raw: RawToken) -> Result<Token, Error> {
    let _ = raw;
    Ok(Token)
}

fn validate_token(raw: RawToken) -> TokenResult {
    let _ = raw;
    Ok(Token)
}

fn convert_qualified(raw: RawToken) -> std::result::Result<Token, Error> {
    let _ = raw;
    Ok(Token)
}

fn parse_token(raw: RawToken) -> Result<Token, Error> {
    let _ = raw;
    Ok(Token)
}

fn try_local_result(raw: RawToken) -> local::Result<Token, Error> {
    let _ = raw;
    local::Result(std::marker::PhantomData)
}

fn try_pair(left: RawToken, right: RawToken) -> Result<Token, Error> {
    let _ = (left, right);
    Ok(Token)
}

impl Token {
    fn make_token(raw: RawToken) -> Result<Self, Error> {
        let _ = raw;
        Ok(Self)
    }

    fn try_refresh(&self, raw: RawToken) -> Result<Self, Error> {
        let _ = raw;
        Ok(Self)
    }
}

struct ConvertedToken;

impl TryFrom<RawToken> for ConvertedToken {
    type Error = Error;

    fn try_from(raw: RawToken) -> Result<Self, Self::Error> {
        let _ = raw;
        Ok(Self)
    }
}

fn try_converted_token(raw: RawToken) -> Result<ConvertedToken, Error> {
    ConvertedToken::try_from(raw)
}

fn main() {}
