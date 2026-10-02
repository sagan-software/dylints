use std::result::Result as ImportedResult;
use std::string::String as Text;

type ErrorText = String;

fn parse_port(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>().map_err(|error| error.to_string())
}

fn validate_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("name is empty".to_string());
    }
    Ok(())
}

fn imported_result(raw: &str) -> ImportedResult<u16, String> {
    raw.parse::<u16>().map_err(|error| error.to_string())
}

fn aliased_string_error(raw: &str) -> Result<u16, ErrorText> {
    raw.parse::<u16>().map_err(|error| error.to_string())
}

fn renamed_string_error(name: &str) -> std::result::Result<(), Text> {
    if name.is_empty() {
        return Err("name is empty".to_string());
    }
    Ok(())
}

fn parse_port_typed(raw: &str) -> Result<u16, std::num::ParseIntError> {
    raw.parse::<u16>()
}

fn borrowed_typed_error() -> Result<std::borrow::Cow<'static, str>, &'static str> {
    Ok(std::borrow::Cow::Borrowed("value"))
}

mod domain_string {
    pub struct String;

    pub fn custom_error_type() -> Result<(), String> {
        Err(String)
    }
}

mod domain_result {
    pub struct Result<T, E>(pub T, pub E);

    pub fn custom_result_type() -> Result<(), std::string::String> {
        Result((), "domain".to_string())
    }
}

fn main() {}
