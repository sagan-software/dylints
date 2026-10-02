#![allow(dead_code)]

fn invalid_retry_budgets() {
    let negative = reqwest::retry::for_host("example.com").max_extra_load(-0.1);
    let _builder = reqwest::Client::builder().retry(negative);

    let too_large = reqwest::retry::for_host("example.com").max_extra_load(1000.1);
    let _builder = reqwest::Client::builder().retry(too_large);
}

fn valid_retry_budgets() {
    let _zero = reqwest::retry::for_host("example.com").max_extra_load(0.0);
    let _bounded = reqwest::retry::for_host("example.com").max_extra_load(0.2);
    let _upper_bound = reqwest::retry::for_host("example.com").max_extra_load(1000.0);
}

fn main() {}
