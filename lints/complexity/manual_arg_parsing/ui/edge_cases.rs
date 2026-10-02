#![allow(dead_code)]

use std::env::{self, args_os};

fn reads_args_with_chain() {
    let _first = std::env::args().skip(1).next();
}

fn reads_args_os_alias() {
    let _all: Vec<_> = args_os().collect();
}

fn local_args() {
    fn args_os() -> Vec<String> {
        Vec::new()
    }

    let _ = args_os();
    let _ = env::vars();
}

fn main() {}
