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

fn main() {}
