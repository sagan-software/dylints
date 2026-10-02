// aux-build:http.rs
extern crate http;

struct Route<'a> {
    method: String,
    http_method: &'a str,
    request_method: &'static str,
    verb: std::string::String,
    method_name: String,
    payment_method: String,
    auth_method: &'a str,
    methodology: String,
    typed_method: Method,
}

trait Client {
    fn send(&self, method: String, auth_method: String) -> String;
    fn verb(&self) -> &'static str;
}

struct Method;

fn http_method(method: String, method_name: String) -> String {
    method
}

fn request_method(verb: &'static str) -> &'static str {
    verb
}

fn payment_method(payment_method: String) -> String {
    payment_method
}

fn typed(method: Method) -> Method {
    method
}

struct HttpClient;

impl Client for HttpClient {
    fn send(&self, method: String, auth_method: String) -> String {
        let _ = auth_method;
        method
    }

    fn verb(&self) -> &'static str {
        "GET"
    }
}

fn closures() {
    let _ = |method: String| method;
}

fn destructured((method, _): (String, u8)) -> String {
    method
}

fn uses_http(_: http::Method) {}

fn main() {}
