#![allow(dead_code)]

use std::convert::TryFrom;
use std::str::FromStr;

struct UserId(String);
struct ParseUserIdError;

mod tracing {
    pub(crate) use crate::warn;
}

#[macro_export]
macro_rules! warn {
    ($($tokens:tt)*) => {};
}

mod local_log {
    macro_rules! warn {
        ($($tokens:tt)*) => {};
    }
    pub(crate) use warn;
}

impl From<String> for UserId {
    fn from(value: String) -> Self {
        tracing::warn!("converting user id");
        Self(value)
    }
}

impl TryFrom<&str> for UserId {
    type Error = ParseUserIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(value.to_owned()))
    }
}

impl FromStr for UserId {
    type Err = ParseUserIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        local_log::warn!("parsing user id");
        Ok(Self(value.to_owned()))
    }
}

fn main() {}
