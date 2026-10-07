// compile-flags: --test --edition 2024
#![allow(dead_code)]

use std::sync::LazyLock;

// A closure is judged by the item that owns it.
static API_ENDPOINT: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("API_ENDPOINT").ok());

fn handler() -> Option<String> {
    let read = || std::env::var("HANDLER_TOKEN").ok();
    read()
}

fn load_config() -> Option<String> {
    let read = || std::env::var("CONFIG_PATH").ok();
    read()
}

#[test]
fn reads_in_harness() {
    let _ = std::env::var("ONLY_IN_TESTS");
}

#[cfg(test)]
mod checks {
    fn helper() -> Option<String> {
        std::env::var("HELPER").ok()
    }
}

#[cfg(all(test, unix))]
fn unix_helper() -> Option<String> {
    std::env::var("UNIX_HELPER").ok()
}

#[cfg(any(test, doc))]
fn doc_or_harness_helper() -> Option<String> {
    std::env::var("DOC_OR_TEST").ok()
}

#[cfg(not(feature = "latest"))]
fn feature_gated_reader() -> Option<String> {
    std::env::var("NOT_TEST").ok()
}

struct Service;

impl Service {
    fn call(&self) {
        let _ = std::env::vars();
        let _ = std::env::var("METHOD_READ");
    }
}
