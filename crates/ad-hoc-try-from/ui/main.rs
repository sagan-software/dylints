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

fn validate_user_id(raw: RawUserId) -> Result<(), Error> {
    let _ = raw;
    Ok(())
}

fn validate_same(raw: UserId) -> Result<UserId, Error> {
    Ok(raw)
}

fn try_generic<T>(value: T) -> Result<UserId, Error> {
    let _ = value;
    Ok(UserId)
}

fn try_foreign(raw: u8) -> Result<u16, Error> {
    Ok(u16::from(raw))
}

struct Wide;

impl From<RawUserId> for Wide {
    fn from(raw: RawUserId) -> Self {
        let _ = raw;
        Self
    }
}

fn try_wide(raw: RawUserId) -> Result<Wide, Error> {
    Ok(Wide::from(raw))
}

trait Builder {
    fn build_user_id(raw: RawUserId) -> Result<UserId, Error>;
}

impl Builder for Wide {
    fn build_user_id(raw: RawUserId) -> Result<UserId, Error> {
        let _ = raw;
        Ok(UserId)
    }
}

impl UserId {
    const KIND: &'static str = "user";
}

fn main() {}
