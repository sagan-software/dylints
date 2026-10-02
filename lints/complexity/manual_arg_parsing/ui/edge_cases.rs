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

fn local_function_pointer() {
    // A local function pointer is not a resolved std::env call at this boundary.
    let arguments = std::env::args;
    let _ = arguments();
}

fn closure_callee() {
    let _ = (|| Vec::<String>::new())();
}
