use std::future::{Future, Ready, ready};

async fn work() -> u8 {
    1
}

fn ready_value() -> Ready<u8> {
    ready(1)
}

fn make_future() -> impl Future<Output = u8> {
    work()
}

fn main() {
    let _ = tokio::task::spawn_blocking(|| async {});
    let _ = tokio::task::spawn_blocking(async || 1_u8);
    let _ = tokio::task::spawn_blocking(make_future);
    let _ = tokio::task::spawn_blocking(ready_value);
    let _ = tokio::task::spawn_blocking(|| {});
    let _ = tokio::task::spawn_blocking(|| 1_u8);
}
