use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Debug)]
struct UserId(String);

#[derive(Debug)]
struct UserIdError;

mod kslog {
    pub fn warn(_message: &str) {}
}

#[macro_export]
macro_rules! error {
    ($($tt:tt)*) => {};
}

#[macro_export]
macro_rules! warn {
    ($($tt:tt)*) => {};
}

mod log {
    pub(crate) use crate::error;
}

mod tracing {
    pub(crate) use crate::warn;
}

impl From<String> for UserId {
    fn from(raw: String) -> Self {
        eprintln!("converting user id: {raw}");
        UserId(raw)
    }
}

impl TryFrom<&str> for UserId {
    type Error = UserIdError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        log::error!("bad user id: {raw}");
        Ok(UserId(raw.to_owned()))
    }
}

impl FromStr for UserId {
    type Err = UserIdError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        tracing::warn!("parsing user id: {raw}");
        Ok(UserId(raw.to_owned()))
    }
}

impl TryFrom<Vec<u8>> for UserId {
    type Error = UserIdError;

    fn try_from(raw: Vec<u8>) -> Result<Self, Self::Error> {
        kslog::warn("converting bytes to user id");
        Ok(UserId(String::from_utf8_lossy(&raw).into_owned()))
    }
}

impl From<&[u8]> for UserId {
    fn from(raw: &[u8]) -> Self {
        UserId(String::from_utf8_lossy(raw).into_owned())
    }
}

fn ordinary_function_logs(raw: &str) -> UserId {
    eprintln!("building user id outside conversion: {raw}");
    UserId(raw.to_owned())
}

trait FromStrLike {
    fn from_str_like(raw: &str) -> Self;
}

impl FromStrLike for UserId {
    fn from_str_like(raw: &str) -> Self {
        eprintln!("local trait can log if it wants: {raw}");
        UserId(raw.to_owned())
    }
}

fn main() {
    let id = UserId::from("alice".as_bytes());
    let _ = &id.0;
    let _ = ordinary_function_logs("bob");
}
