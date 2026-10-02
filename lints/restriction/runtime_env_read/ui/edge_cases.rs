#![allow(dead_code)]

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

#[test]
fn can_read_env_in_tests() {
    let _ = env::var("TEST_ONLY");
}

fn main() {}
