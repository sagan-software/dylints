#![allow(dead_code)]

use std::sync::Arc;

fn overridden_provider() -> Result<reqwest::Client, reqwest::Error> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    reqwest::Client::builder()
        .cookie_provider(jar)
        .cookie_store(true)
        .build()
}

fn valid_provider() -> Result<reqwest::Client, reqwest::Error> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    reqwest::Client::builder().cookie_provider(jar).build()
}

fn valid_provider_last() -> Result<reqwest::Client, reqwest::Error> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    reqwest::Client::builder()
        .cookie_store(true)
        .cookie_provider(jar)
        .build()
}

fn main() {}
