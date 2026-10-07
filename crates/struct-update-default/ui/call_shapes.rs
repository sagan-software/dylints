#![allow(dead_code)]

#[derive(Default)]
struct Request {
    timeout: u64,
    retries: u8,
}

impl Request {
    fn new() -> Self {
        Self {
            timeout: 30,
            retries: 3,
        }
    }
}

#[derive(Default)]
struct Wrapper<T> {
    value: T,
    count: usize,
}

// A bound on the whole struct does not identify a concrete default implementation.
fn generic_bound<T>(value: T) -> Wrapper<T>
where
    Wrapper<T>: Default,
{
    Wrapper {
        value,
        ..Default::default()
    }
}

// Closure and inherent constructor bases must not receive default-field rewrites.
fn main() {
    let _closure = Request {
        timeout: 5,
        ..(|| Request::default())()
    };
    let _inherent = Request {
        timeout: 5,
        ..Request::new()
    };
}
