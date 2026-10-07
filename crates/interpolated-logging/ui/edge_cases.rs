#![allow(dead_code)]

macro_rules! error {
    ($($tokens:tt)*) => {};
}

macro_rules! trace {
    ($($tokens:tt)*) => {};
}

fn logs(user_id: u64, status: &str) {
    error!("user {user_id} failed with {status}");
    trace!("literal braces {{not a placeholder}}");
    error!("user {} failed with {}", user_id, status);
}

fn main() {}
