struct Error;

mod ids {
    pub struct RawUserId;
    pub struct UserId;

    pub type UserIdAlias = UserId;

    impl From<RawUserId> for UserId {
        fn from(raw: RawUserId) -> Self {
            let _ = raw;
            Self
        }
    }

    impl UserId {
        fn make_associated_user_id(raw: RawUserId) -> Self {
            let _ = raw;
            Self
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

fn main() {}
