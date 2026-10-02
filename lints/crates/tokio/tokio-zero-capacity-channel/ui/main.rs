#![allow(dead_code)]

use tokio::sync::{broadcast, mpsc};

fn invalid_capacities() {
    let _ = mpsc::channel::<u8>(0);
    let _ = broadcast::channel::<u8>(0);
}

fn valid_capacities() {
    let _ = mpsc::channel::<u8>(1);
    let _ = broadcast::channel::<u8>(16);
}

mod other {
    pub fn channel(_capacity: usize) {}
}

fn similarly_named_user_function() {
    other::channel(0);
}

fn main() {}
