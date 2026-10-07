struct ServiceConfig<'a> {
    callback_url: String,
    webhook_uri: &'a str,
    endpoint: &'static str,
    display_name: String,
    request_url: Url,
}

struct Url;

fn main() {}

struct OptionalServiceConfig {
    callback_url: Option<String>,
    typed_url: Option<Url>,
}
