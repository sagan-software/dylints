#![allow(dead_code)]

use clap::Arg;

fn conflicting_requirements() {
    let _ = Arg::new("config")
        .required(true)
        .required_if_eq("mode", "custom");
    let _ = Arg::new("config")
        .required_unless_present("defaults")
        .required(true);
    let _ = Arg::new("config")
        .required(true)
        .required_unless_present_any(["defaults", "automatic"]);
}

fn valid_requirements() {
    let _ = Arg::new("config").required(true);
    let _ = Arg::new("config").required_if_eq("mode", "custom");
    let _ = Arg::new("config")
        .required(true)
        .required_if_eq("mode", "custom")
        .required(false);
    let _ = Arg::new("config")
        .required(true)
        .required_unless_present_any([] as [&str; 0]);
}

fn computed_conditions(mode: &'static str) {
    let _ = Arg::new("config")
        .required(true)
        .required_if_eq(mode, "custom");
    let _ = Arg::new("config")
        .required_unless_present(mode)
        .required(true);
}

fn main() {}
