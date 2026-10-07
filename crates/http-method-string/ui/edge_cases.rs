// aux-build:http.rs
#![allow(dead_code)]

extern crate http;

use std::string::String as StdString;

struct Method;
type MethodText = std::string::String;
type MethodStr = str;
struct MethodTextWrapper(String);

struct Request<'a> {
    method: MethodText,
    http_verb: &'a str,
    request_method: std::string::String,
    http_method: &'a MethodStr,
    verb: StdString,
    payment_method: String,
    typed_method: Method,
    wrapped_method: MethodTextWrapper,
}

trait Client {
    fn send(&self, method: MethodText) -> MethodText;
}

fn method_name(method: MethodText) -> MethodText {
    method
}

fn http_method(method: std::string::String) -> std::string::String {
    method
}

fn request_method(verb: &'static MethodStr) -> &'static MethodStr {
    verb
}

fn typed(method: Method) -> Method {
    method
}

fn main() {}
