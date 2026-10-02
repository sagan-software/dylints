#![allow(dead_code)]

use clap::{Arg, ValueHint};

fn invalid_options() {
    let _ = Arg::new("command")
        .long("command")
        .value_hint(ValueHint::CommandWithArguments);
    let _ = Arg::new("command")
        .value_hint(ValueHint::CommandWithArguments)
        .short('c');
}

fn valid_arguments() {
    let _ = Arg::new("command").value_hint(ValueHint::CommandWithArguments);
    let _ = Arg::new("file")
        .long("file")
        .value_hint(ValueHint::FilePath);
    let _ = Arg::new("command")
        .long("command")
        .value_hint(ValueHint::CommandWithArguments)
        .value_hint(ValueHint::CommandName);
}

fn computed_name(name: &'static str) {
    let _ = Arg::new("command")
        .long(name)
        .value_hint(ValueHint::CommandWithArguments);
}

fn removed_name() {
    let _ = Arg::new("command")
        .long("command")
        .long(None)
        .value_hint(ValueHint::CommandWithArguments);
}

fn main() {}
