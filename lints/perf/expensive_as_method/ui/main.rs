struct ParseError;

struct Token {
    raw: String,
    bytes: Vec<u8>,
}

impl Token {
    fn as_str(&self) -> &str {
        &self.raw
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn as_string(&self) -> String {
        self.raw.clone()
    }

    fn as_number(&self) -> Result<u64, ParseError> {
        self.raw.parse().map_err(|_| ParseError)
    }

    fn as_optional_number(&self) -> Option<u64> {
        self.raw.parse().ok()
    }

    fn as_display_value(&self) -> String {
        format!("token: {}", self.raw)
    }

    fn as_decoded(&self) -> Vec<u8> {
        decode(&self.raw)
    }

    fn to_string_value(&self) -> String {
        self.raw.clone()
    }
}

trait Render {
    fn as_string(&self) -> String;
}

impl Render for Token {
    fn as_string(&self) -> String {
        self.raw.clone()
    }
}

fn decode(raw: &str) -> Vec<u8> {
    raw.as_bytes().to_vec()
}

fn main() {}
