#![allow(dead_code)]

use std::str::FromStr;

struct Error;
struct UserId;
struct Token;
struct ProjectId;
struct SourceText;

type RawText<'a> = &'a str;
type ParseResult<T> = std::result::Result<T, Error>;

fn parse_user_id(raw: &str) -> Result<UserId, Error> {
    let _ = raw;
    Ok(UserId)
}

fn parse_project_id(raw: &str) -> std::result::Result<ProjectId, Error> {
    let _ = raw;
    Ok(ProjectId)
}

fn from_str_token(raw: RawText<'_>) -> ParseResult<Token> {
    let _ = raw;
    Ok(Token)
}

impl FromStr for UserId {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let _ = raw;
        Ok(UserId)
    }
}

impl Token {
    fn parse_token(raw: RawText<'_>) -> ParseResult<Self> {
        let _ = raw;
        Ok(Token)
    }

    fn parse_with_cache(&mut self, raw: &str) -> ParseResult<Self> {
        let _ = raw;
        Ok(Token)
    }
}

fn parse_user_id_with_prefix(raw: &str, prefix: &str) -> Result<UserId, Error> {
    let _ = (raw, prefix);
    Ok(UserId)
}

fn parse_user_id_lossy(raw: &str) -> Result<UserId, Error> {
    let _ = raw;
    Ok(UserId)
}

fn parse_and_cache_user_id(raw: &str) -> Result<UserId, Error> {
    let _ = raw;
    Ok(UserId)
}

fn parse_source(raw: SourceText) -> Result<UserId, Error> {
    let _ = raw;
    Ok(UserId)
}

fn parse_status(raw: &str) -> Result<(), Error> {
    let _ = raw;
    Ok(())
}

fn parse_count(raw: &str) -> usize {
    raw.len()
}

fn main() {}
