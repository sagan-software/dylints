struct SecretString(String);

struct SecretBox<T: ?Sized>(Box<T>);

struct Credentials<'a> {
    api_key: String,
    session_token: &'a str,
    password: Vec<u8>,
    secret: &'a [u8],
    access_token: [u8; 32],
    token_count: usize,
    typed_api_key: SecretString,
    boxed_password: SecretBox<[u8]>,
}

trait Client {
    fn send(&self, refresh_token: String, request_id: String);
}

fn authenticate(access_token: String, password: &[u8], username: String) {
    let session_token = "raw-token";
    let api_key = vec![1_u8, 2, 3];
    let secret = [0_u8; 32];
    let typed_secret = SecretString(String::new());
    let token_count = 3_usize;

    let _ = (
        access_token,
        password,
        username,
        session_token,
        api_key,
        secret,
        typed_secret,
        token_count,
    );
}

fn main() {}

struct OptionalCredentials {
    password: Option<String>,
    typed_password: Option<SecretString>,
}
