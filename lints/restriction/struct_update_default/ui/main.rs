#![allow(dead_code)]

#[derive(Default)]
struct Request {
    timeout: u64,
    retries: u8,
}

fn main() {
    let _request = Request {
        timeout: 5,
        ..Default::default()
    };
}
