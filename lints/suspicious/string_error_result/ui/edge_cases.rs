#![allow(dead_code)]

type StdResult<T, E> = std::result::Result<T, E>;
type ErrorText = String;
type StringResult<T> = std::result::Result<T, String>;

fn owned_error() -> Result<(), String> {
    Err("failed".to_owned())
}

fn nested_result() -> std::result::Result<Result<u8, String>, String> {
    Ok(Ok(1))
}

fn alias_result() -> StdResult<(), String> {
    Err("alias".to_owned())
}

fn alias_error_result() -> StdResult<(), ErrorText> {
    Err("alias error".to_owned())
}

fn baked_string_result() -> StringResult<()> {
    Err("baked alias".to_owned())
}

fn typed_error() -> Result<(), std::io::Error> {
    Ok(())
}

mod domain_result {
    pub struct Result<T, E>(pub T, pub E);

    pub fn custom_result_alias() -> Result<(), std::string::String> {
        Result((), "not std result".to_string())
    }
}

mod domain_string {
    pub struct String;

    pub fn custom_string_error() -> std::result::Result<(), String> {
        Err(String)
    }
}

fn main() {}
