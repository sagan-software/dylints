struct StatusCode(u16);

enum HttpMethod {
    Get,
    Post,
}

type Url = String;

struct EmailAddress(String);

struct PathBuf(String);

type Duration = u64;

type Millis = u64;
type Instant = Millis;

struct PaymentMethod(String);

struct RouteMethod(String);

struct HttpClient;

fn accepts_standard_types(
    _path: std::path::PathBuf,
    _duration: std::time::Duration,
    _instant: std::time::Instant,
) {
}

fn main() {}
