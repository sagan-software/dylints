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
