#![allow(dead_code)]

fn unbudgeted_policy() {
    let policy = reqwest::retry::for_host("example.com").no_budget();
    let _builder = reqwest::Client::builder().retry(policy);
}

fn valid_default_budget() {
    let policy = reqwest::retry::for_host("example.com");
    let _builder = reqwest::Client::builder().retry(policy);
}

fn valid_bounded_budget() {
    let policy = reqwest::retry::for_host("example.com").max_extra_load(0.2);
    let _builder = reqwest::Client::builder().retry(policy);
}

fn main() {}
