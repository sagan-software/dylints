#![allow(dead_code)]

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};

async fn invalid_calls() {
    let mutex = Mutex::new(1_u8);
    let _guard = mutex.blocking_lock();

    let rwlock = RwLock::new(1_u8);
    let _read_guard = rwlock.blocking_read();
    let _write_guard = rwlock.blocking_write();

    let (sender, mut receiver) = mpsc::channel(1);
    let _ = sender.blocking_send(1_u8);
    let _ = receiver.blocking_recv();
}

fn valid_sync_call(mutex: &Mutex<u8>) {
    let _guard = mutex.blocking_lock();
}

async fn valid_spawn_blocking_call(mutex: Arc<Mutex<u8>>) {
    let _ = tokio::task::spawn_blocking(move || {
        let _guard = mutex.blocking_lock();
    })
    .await;
}

struct OtherMutex;

impl OtherMutex {
    fn blocking_lock(&self) {}
}

async fn similarly_named_user_method() {
    OtherMutex.blocking_lock();
}

fn main() {}
