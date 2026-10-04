#![allow(dead_code)]

use std::env::{vars as read_vars, vars_os as read_vars_os};

#[expect(runtime_env_read, reason = "std::env::vars must be reported in processing code")]
fn reads_vars() {
    let _ = std::env::vars();
}

#[expect(runtime_env_read, reason = "std::env::vars_os must be reported in processing code")]
fn reads_vars_os() {
    let _ = std::env::vars_os();
}

#[expect(runtime_env_read, reason = "an imported std::env::vars must still be resolved")]
fn reads_imported_vars() {
    let _ = read_vars();
}

#[expect(runtime_env_read, reason = "an imported std::env::vars_os must still be resolved")]
fn reads_imported_vars_os() {
    let _ = read_vars_os();
}

mod fake_env {
    pub fn vars() {}
    pub fn vars_os() {}
}

fn resolved_lookalikes_are_quiet() {
    fake_env::vars();
    fake_env::vars_os();
}

fn local_lookalikes_are_quiet() {
    fn vars() {}
    fn vars_os() {}

    vars();
    vars_os();
}

fn unrelated_std_calls_are_quiet() {
    let _ = std::env::args();
    let _ = std::env::current_dir();
}

fn non_path_callees_are_quiet() {
    (|| {})();
}

fn main() {}
