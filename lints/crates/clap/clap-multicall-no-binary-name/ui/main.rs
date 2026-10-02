#![allow(dead_code)]

use clap::Command;

fn incompatible() {
    let _ = Command::new("busybox").multicall(true).no_binary_name(true);

    let _ = Command::new("busybox").no_binary_name(true).multicall(true);
}

fn compatible() {
    let _ = Command::new("busybox").multicall(true);
    let _ = Command::new("daemon").no_binary_name(true);
    let _ = Command::new("busybox")
        .no_binary_name(true)
        .multicall(true)
        .no_binary_name(false);
}

struct OtherCommand;

impl OtherCommand {
    fn multicall(self, _enabled: bool) -> Self {
        self
    }

    fn no_binary_name(self, _enabled: bool) -> Self {
        self
    }
}

fn similarly_named_user_methods() {
    let _ = OtherCommand.multicall(true).no_binary_name(true);
}

fn main() {}
