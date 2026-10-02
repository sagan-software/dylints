#![allow(dead_code)]

use tokio::runtime::Builder;

fn invalid_thread_counts() {
    let _ = Builder::new_multi_thread().worker_threads(0).build();
    let _ = Builder::new_multi_thread().max_blocking_threads(0).build();
}

fn valid_thread_counts() {
    let _ = Builder::new_multi_thread().worker_threads(2).build();
    let _ = Builder::new_multi_thread().max_blocking_threads(16).build();
    let _ = Builder::new_multi_thread().build();
}

struct OtherBuilder;

impl OtherBuilder {
    fn worker_threads(self, _count: usize) -> Self {
        self
    }
}

fn similarly_named_user_method() {
    let _ = OtherBuilder.worker_threads(0);
}

fn main() {}
