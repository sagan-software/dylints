use clap::{Arg, ArgAction};

fn action() -> ArgAction {
    ArgAction::Set
}

fn main() {
    let _ = Arg::new("bad").allow_hyphen_values(true);
    let _ = Arg::new("flag")
        .action(ArgAction::SetTrue)
        .allow_hyphen_values(true);
    let _ = Arg::new("good").allow_hyphen_values(true).num_args(1);
    let _ = Arg::new("set")
        .action(ArgAction::Set)
        .allow_hyphen_values(true);
    let _ = Arg::new("append")
        .allow_hyphen_values(true)
        .action(clap::ArgAction::Append);
    let _ = Arg::new("disabled")
        .allow_hyphen_values(true)
        .allow_hyphen_values(false);
    let local = ArgAction::Set;
    let _ = Arg::new("local").action(local).allow_hyphen_values(true);
    let _ = Arg::new("computed")
        .action(action())
        .allow_hyphen_values(true);
}
