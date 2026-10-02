struct Error;
struct RawUserId;
struct UserId;

fn make_user_id(raw: RawUserId) -> Result<UserId, Error> {
    let _ = raw;
    Ok(UserId)
}

fn map_user_id(raw: RawUserId) -> Result<UserId, Error> {
    let _ = raw;
    Ok(UserId)
}

fn map_optional_text(raw: &str) -> Result<Option<UserId>, Error> {
    let _ = raw;
    Ok(Some(UserId))
}

fn make_pair(left: RawUserId, right: RawUserId) -> Result<UserId, Error> {
    let _ = (left, right);
    Ok(UserId)
}

fn describe(raw: RawUserId) -> UserId {
    let _ = raw;
    UserId
}

fn main() {}
