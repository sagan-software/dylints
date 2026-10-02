#![allow(dead_code)]

use tracing::{info_span, warn};

struct Request {
    error: std::io::Error,
}

fn invalid_string_fields(error: std::io::Error, request: Request) {
    warn!(error = error.to_string(), "request failed");
    info_span!("request", failure = request.error.to_string());
}

fn valid_fields(error: std::io::Error, request: Request) {
    warn!(error = %error, "request failed");
    info_span!("request", failure = %request.error);
    warn!(owned = String::from("already owned"));
}

struct InherentString;

impl InherentString {
    fn to_string(&self) -> String {
        String::from("custom")
    }
}

fn valid_inherent_method(value: InherentString) {
    warn!(value = value.to_string());
}

fn main() {}
