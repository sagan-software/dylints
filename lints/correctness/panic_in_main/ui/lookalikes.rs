#![allow(dead_code)]

// Local macros and methods that share a panicking name do not panic.
macro_rules! assert {
    ($($tokens:tt)*) => {
        let _ = stringify!($($tokens)*);
    };
}

macro_rules! todo {
    () => {
        ()
    };
}

struct Lock;

impl Lock {
    fn unwrap(&self) -> u8 {
        1
    }

    fn expect(&self, _message: &str) -> u8 {
        2
    }
}

fn main() {
    let lock = Lock;
    let _first = lock.unwrap();
    let _second = lock.expect("never panics");
    assert!(false);
    todo!();
    let _total = Ok::<u8, ()>(1).unwrap_or_default();
    if std::env::var_os("PANIC_IN_MAIN").is_some() {
        unreachable!("standard macro");
    }
}
