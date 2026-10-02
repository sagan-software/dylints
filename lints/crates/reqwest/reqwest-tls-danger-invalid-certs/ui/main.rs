#![allow(dead_code)]

fn invalid_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_certs(true)
        .build()
}

fn valid_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_certs(false)
        .build()
}

fn configurable_client(enabled: bool) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_certs(enabled)
        .build()
}

fn main() {}
