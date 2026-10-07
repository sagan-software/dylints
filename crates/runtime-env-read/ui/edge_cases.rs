#![allow(dead_code)]
#![allow(non_ascii_idents)]

use std::env::{self, var_os};

fn request_handler() -> Result<String, std::env::VarError> {
    env::var("API_TOKEN")
}

fn render_template() -> Option<std::ffi::OsString> {
    var_os("TEMPLATE_PATH")
}

fn load_config() -> String {
    std::env::var("API_TOKEN").unwrap_or_default()
}

struct AppConfig;

impl AppConfig {
    fn load() -> String {
        std::env::var("APP_CONFIG").unwrap_or_default()
    }
}

struct APIConfig;

impl APIConfig {
    fn load() -> String {
        std::env::var("API_CONFIG").unwrap_or_default()
    }
}

struct Configé;

impl Configé {
    fn load() -> String {
        std::env::var("UNICODE_BOUNDARY").unwrap_or_default()
    }
}

#[test]
fn can_read_env_in_tests() {
    let _ = env::var("TEST_ONLY");
}

fn main() {}
