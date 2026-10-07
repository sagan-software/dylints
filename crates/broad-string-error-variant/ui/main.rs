enum ConfigError<'a> {
    Invalid { message: String },
    Denied { details: &'a str },
    Reason(String),
    Error(&'a str),
}

enum ApiErrors {
    Failed { reason: String },
}

enum Event {
    Message(String),
    Error(String),
}

enum TypedError {
    Parse { source: std::num::ParseIntError },
    Redacted { error_code: RedactedCode },
    Message(DomainMessage),
    Details(String, String),
}

struct RedactedCode(String);
struct DomainMessage(String);

fn main() {}

enum OptionalError {
    Invalid { message: Option<String> },
    Typed { message: Option<DomainMessage> },
}
