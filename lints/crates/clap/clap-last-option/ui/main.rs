#![allow(dead_code)]

use clap::Arg;

fn ineffective() {
    let _ = Arg::new("input").long("input").last(true);
    let _ = Arg::new("output").last(true).short('o');
}

fn effective_or_disabled() {
    let _ = Arg::new("input").last(true);
    let _ = Arg::new("output").long("output");
    let _ = Arg::new("output").long("output").last(true).last(false);
}

struct OtherArg;

impl OtherArg {
    fn last(self, _enabled: bool) -> Self {
        self
    }

    fn long(self, _name: &str) -> Self {
        self
    }
}

fn similarly_named_user_methods() {
    let _ = OtherArg.long("input").last(true);
}

fn main() {}
