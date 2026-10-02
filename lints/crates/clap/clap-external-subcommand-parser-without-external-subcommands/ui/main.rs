use std::ffi::OsString;

use clap::{Command, value_parser};

fn main() {
    let _ = Command::new("bad").external_subcommand_value_parser(value_parser!(OsString));
    let _ = Command::new("good")
        .external_subcommand_value_parser(value_parser!(OsString))
        .allow_external_subcommands(true);
}
