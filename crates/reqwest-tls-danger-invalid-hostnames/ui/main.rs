#![allow(dead_code)]

const ACCEPT_INVALID: bool = true;
const VALIDATION_ENABLED: bool = false;

fn invalid_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(true)
        .build()
}

fn invalid_client_with_constant() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(ACCEPT_INVALID)
        .build()
}

#[allow(deprecated)]
fn invalid_client_with_deprecated_constant() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .danger_accept_invalid_hostnames(ACCEPT_INVALID)
        .build()
}

fn valid_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(false)
        .build()
}

fn valid_client_with_constant() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(VALIDATION_ENABLED)
        .build()
}

fn configurable_client(enabled: bool) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(enabled)
        .build()
}

fn runtime_flag() -> bool {
    panic!("the UI fixture does not run this function")
}

fn invalid_client_with_negation() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(!VALIDATION_ENABLED)
        .build()
}

fn invalid_client_with_conjunction() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(ACCEPT_INVALID && (true || runtime_flag()))
        .build()
}

fn invalid_client_with_disjunction() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(VALIDATION_ENABLED || ACCEPT_INVALID)
        .build()
}

fn invalid_client_with_short_circuit() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(true || runtime_flag())
        .build()
}

fn valid_client_with_short_circuit() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(false && runtime_flag())
        .build()
}

fn valid_client_with_unknown_expression() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(true && runtime_flag())
        .build()
}

fn valid_client_with_unknown_disjunction() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(false || runtime_flag())
        .build()
}

fn valid_client_with_unsupported_comparison() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(true == false)
        .build()
}

fn valid_client_with_unknown_negation() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(!runtime_flag())
        .build()
}

fn bounded_expression_is_clean() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(!(!(!(!(!(!(!(!(!(!(!(!(!(!(!(!(true)))))))))))))))))
        .build()
}

struct LocalBuilder;

impl LocalBuilder {
    fn tls_danger_accept_invalid_hostnames(self, _enabled: bool) {}
}

fn local_method_is_clean() {
    LocalBuilder.tls_danger_accept_invalid_hostnames(ACCEPT_INVALID);
}

mod unrelated_api {
    pub struct Builder;

    pub trait TlsMethods {
        fn danger_accept_invalid_hostnames(self, enabled: bool);
    }

    impl TlsMethods for Builder {
        fn danger_accept_invalid_hostnames(self, _enabled: bool) {}
    }
}

use unrelated_api::{Builder as ImportedBuilder, TlsMethods as _};

fn imported_method_is_clean() {
    ImportedBuilder.danger_accept_invalid_hostnames(ACCEPT_INVALID);
}

use self::ACCEPT_INVALID as IMPORTED_ACCEPT_INVALID;

fn invalid_client_with_renamed_constant() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(IMPORTED_ACCEPT_INVALID)
        .build()
}

macro_rules! invalid_flag {
    () => {
        ACCEPT_INVALID
    };
}

fn invalid_client_with_macro_constant() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(invalid_flag!())
        .build()
}

fn valid_client_with_shadowed_constant() -> Result<reqwest::Client, reqwest::Error> {
    const ACCEPT_INVALID: bool = false;
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(ACCEPT_INVALID)
        .build()
}

fn main() {}
