#![allow(dead_code)]

use std::env::args_os as imported_args_os;
use std::env::{self, args_os};
use std::process::Command as ChildCommand;

fn reads_args_with_chain() {
    let _first = std::env::args().skip(1).next();
}

fn reads_args_os_alias() {
    let _all: Vec<_> = args_os().collect();
}

fn forwards_arguments_to_child() {
    let _command = std::process::Command::new("child").args(std::env::args_os().skip(1));
}

fn counts_arguments_without_parsing() {
    let _count = std::env::args().count();
}

fn forwards_renamed_imports() {
    let _command = ChildCommand::new("child").args(imported_args_os().skip(1).take(8));
}

fn counts_four_bounded_adapters() {
    let _count = imported_args_os().skip(1).take(8).skip(2).take(3).count();
}

fn warns_after_five_bounded_adapters() {
    let _count = std::env::args_os()
        .skip(1)
        .take(8)
        .skip(2)
        .take(3)
        .skip(1)
        .count();
}

struct LocalCommand;

impl LocalCommand {
    fn args<I>(&self, _arguments: I) {}

    fn count<I>(&self, _arguments: I) {}
}

fn calls_unrelated_args_and_count_methods() {
    LocalCommand.args(std::env::args_os().skip(1));
    LocalCommand.count(std::env::args().skip(1));
}

fn forwards_a_stored_iterator_later() {
    let arguments = std::env::args_os().skip(1);
    let _command = std::process::Command::new("child").args(arguments);
}

fn filters_before_forwarding() {
    let _command = std::process::Command::new("child").args(std::env::args_os().filter(|_| true));
}

fn mixes_arguments_before_forwarding() {
    let _command = std::process::Command::new("child")
        .args(std::env::args_os().chain(std::iter::once(std::ffi::OsString::from("--extra"))));
}

fn passes_arguments_to_an_unrelated_function() {
    std::mem::drop(std::env::args());
}

fn drops_arguments_without_parsing() {
    let _ = std::env::args();
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
