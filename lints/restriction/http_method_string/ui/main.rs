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

fn main() {}
