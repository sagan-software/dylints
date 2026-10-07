// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]
#![feature(default_field_values)]

#[derive(Default)]
struct Request {
    timeout: u64,
    retries: u8,
    name: String,
}

#[derive(Default)]
struct Pair(u8, u16);

#[derive(Default)]
struct Wrapper<T> {
    value: T,
    count: usize,
}

struct Manual {
    timeout: u64,
    retries: u8,
}

impl Default for Manual {
    fn default() -> Self {
        Self {
            timeout: 30,
            retries: 3,
        }
    }
}

#[derive(Default)]
struct WithFieldDefault {
    timeout: u64 = 30,
    retries: u8,
}

macro_rules! request {
    ($timeout:expr) => {
        Request {
            timeout: $timeout,
            ..Default::default()
        }
    };
}

fn main() {
    let _derived = Request {
        timeout: 5,
        ..Default::default()
    };
    let _type_relative = Request {
        retries: 2,
        ..Request::default()
    };
    let _all_written = Request {
        timeout: 5,
        retries: 2,
        name: String::new(),
        ..Default::default()
    };
    #[rustfmt::skip]
    let _one_line = Request { timeout: 1, ..Default::default() };
    let _tuple = Pair {
        0: 1,
        ..Default::default()
    };
    let _generic = Wrapper::<String> {
        count: 1,
        ..Default::default()
    };

    // These cases keep help-only output because the rewrite could change values.
    let _manual = Manual {
        timeout: 5,
        ..Default::default()
    };
    let _field_default = WithFieldDefault {
        retries: 1,
        ..Default::default()
    };
    let _commented = Request {
        timeout: 5,
        .. // keep the remaining defaults
        Default::default()
    };
    let _macro = request!(5);

    // Other update bases are not reported.
    let base = Request::default();
    let _from_base = Request { timeout: 1, ..base };
}
