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

struct ParsedId;

impl FromStr for ParsedId {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let _ = raw;
        Ok(ParsedId)
    }
}

fn parse_parsed_id(raw: &str) -> Result<ParsedId, Error> {
    raw.parse()
}

struct Borrowed<'a>(&'a str);

fn parse_borrowed(raw: &str) -> Result<Borrowed<'_>, Error> {
    Ok(Borrowed(raw))
}

fn parse_foreign(raw: &str) -> Result<String, Error> {
    Ok(raw.to_owned())
}

trait Parser {
    fn parse_value(raw: &str) -> Result<UserId, Error>;
}

impl Parser for Token {
    fn parse_value(raw: &str) -> Result<UserId, Error> {
        let _ = raw;
        Ok(UserId)
    }
}

impl Token {
    const KIND: &'static str = "token";

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
