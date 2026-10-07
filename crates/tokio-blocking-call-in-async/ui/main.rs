#![allow(dead_code)]

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast, mpsc, oneshot};

async fn invalid_calls() {
    let mutex = Mutex::new(1_u8);
    let _guard = mutex.blocking_lock();

    let rwlock = RwLock::new(1_u8);
    let _read_guard = rwlock.blocking_read();
    let _write_guard = rwlock.blocking_write();

    let (sender, mut receiver) = mpsc::channel(1);
    let _ = sender.blocking_send(1_u8);
    let _ = receiver.blocking_recv();
    let mut buffer = Vec::new();
    let _ = receiver.blocking_recv_many(&mut buffer, 4);

    let (_tx, rx) = oneshot::channel::<u8>();
    let _ = rx.blocking_recv();

    let (_tx, mut rx) = broadcast::channel::<u8>(1);
    let _ = rx.blocking_recv();
}

async fn invalid_owned_lock(mutex: Arc<Mutex<u8>>) {
    let _guard = mutex.blocking_lock_owned();
}

fn valid_sync_call(mutex: &Mutex<u8>) {
    let _guard = mutex.blocking_lock();
}

async fn valid_nested_sync_function() {
    fn nested(mutex: &Mutex<u8>) {
        let _guard = mutex.blocking_lock();
    }
    nested(&Mutex::new(1));
}

async fn valid_spawn_blocking_call(mutex: Arc<Mutex<u8>>) {
    let _ = tokio::task::spawn_blocking(move || {
        let _guard = mutex.blocking_lock();
    })
    .await;
}

async fn valid_async_call(mutex: &Mutex<u8>) {
    let _guard = mutex.lock().await;
}

struct OtherMutex;

impl OtherMutex {
    fn blocking_lock(&self) {}
}

async fn similarly_named_user_method() {
    OtherMutex.blocking_lock();
}

fn main() {}
