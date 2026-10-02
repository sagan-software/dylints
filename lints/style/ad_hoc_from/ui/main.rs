struct Error;

mod ids {
    pub struct RawUserId;
    pub struct UserId;

    pub type UserIdAlias = UserId;

    impl UserId {
        pub const PREFIX: &'static str = "user";

        fn make_associated_user_id(raw: RawUserId) -> Self {
            let _ = raw;
            Self
        }

        fn build_request(&self) -> RawUserId {
            RawUserId
        }
    }
}

type UserIdAlias = ids::UserId;

fn make_user_id(raw: ids::RawUserId) -> ids::UserId {
    let _ = raw;
    ids::UserId
}

fn convert_user_id_alias(raw: ids::RawUserId) -> UserIdAlias {
    let _ = raw;
    ids::UserId
}

fn from_qualified_user_id(raw: ids::RawUserId) -> crate::ids::UserIdAlias {
    let _ = raw;
    ids::UserId
}

fn extract_user_id(raw: ids::RawUserId) -> ids::UserId {
    let _ = raw;
    ids::UserId
}

fn extract_external_text(raw: &str) -> String {
    raw.to_owned()
}

fn convert_and_log_user_id(raw: ids::RawUserId) -> ids::UserId {
    let _ = raw;
    println!("converted user id");
    ids::UserId
}

fn compare_user_ids(left: ids::RawUserId, right: ids::RawUserId) -> ids::UserId {
    let _ = (left, right);
    ids::UserId
}

fn make_same_user_id(raw: ids::RawUserId) -> ids::RawUserId {
    raw
}

fn make_optional_user_id(raw: ids::RawUserId) -> Option<ids::UserId> {
    let _ = raw;
    Some(ids::UserId)
}

fn make_checked_user_id(raw: ids::RawUserId) -> Result<ids::UserId, Error> {
    let _ = raw;
    Ok(ids::UserId)
}

struct Token;
struct ConvertedToken;

impl From<Token> for ConvertedToken {
    fn from(token: Token) -> Self {
        let _ = token;
        Self
    }
}

fn make_converted_token(token: Token) -> ConvertedToken {
    token.into()
}

struct Port(u16);

fn make_port_fixture(raw: &str) -> Port {
    Port(raw.parse().unwrap())
}

fn make_port_checked(raw: &str) -> Port {
    Port(raw.parse().expect("valid port"))
}

fn make_port_asserted(raw: u16) -> Port {
    assert!(raw > 0, "port must be positive");
    Port(raw)
}

fn make_port_or_panic(raw: i32) -> Port {
    if raw < 0 {
        panic!("negative port");
    }
    Port(0)
}

fn make_port_lossy(raw: Option<u16>) -> Port {
    Port(raw.unwrap_or_default())
}

fn make_port_plain(raw: u8) -> Port {
    Port(u16::from(raw))
}

fn make_port_option(raw: Option<u16>) -> Port {
    let _ = raw;
    Port(0)
}

fn make_generic<T>(value: T) -> Port {
    let _ = value;
    Port(0)
}

fn make_from_unit(value: ()) -> Port {
    let _ = value;
    Port(0)
}

trait Factory {
    fn make_port(raw: u16) -> Port;
}

impl Factory for Token {
    fn make_port(raw: u16) -> Port {
        Port(raw)
    }
}

fn main() {}
