#![allow(dead_code, unused_variables)]

struct SecretString(String);

struct ApiRequest<'a> {
    bearer_token: String,
    refresh_secret: &'a str,
    password_hash: Vec<u8>,
    token_count: usize,
    typed_token: SecretString,
}

fn send(api_key: String, session_secret: &[u8], retry_count: usize) {
    let password = "raw password".to_owned();
    let secret_count = retry_count;
    let _ = (api_key, session_secret, password, secret_count);
}

fn main() {}

type SecretAlias = String;

trait Login {
    fn login(&self, password: SecretAlias, _: (u8, u8));
    fn rotate(&self, api_key: &[u8]) {}
}

struct Service;

// The trait fixes these parameter types, so only the trait declaration is reported.
impl Login for Service {
    fn login(&self, password: SecretAlias, _: (u8, u8)) {}
}

struct UnsizedSecret {
    secret: [u8],
}

struct PositionalToken(String);

struct BoxedSecrets {
    secret_bytes: Box<[u8]>,
    secret_text: Box<str>,
}

fn closures_and_patterns() {
    let _ = |password: String| password;
    let (token, _) = (String::new(), 1_u8);
    for secret in [String::new()] {}
}
