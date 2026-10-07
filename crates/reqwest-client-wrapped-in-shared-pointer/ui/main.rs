#![allow(dead_code)]

use std::{rc::Rc, sync::Arc};

fn wrapped_clients() {
    let client = reqwest::Client::new();
    let _shared = Arc::new(client);

    let client = reqwest::Client::new();
    let _local = Rc::new(client);
}

fn valid_clone() {
    let client = reqwest::Client::new();
    let _worker_client = client.clone();
}

fn valid_other_shared_value() {
    let _value = Arc::new(String::from("not a client"));
}

fn main() {}
