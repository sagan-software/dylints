use clap::{Arg, ArgAction};

fn main() {
    let _ = Arg::new("bad").require_equals(true);
    let _ = Arg::new("good").require_equals(true).num_args(1);
    let _ = Arg::new("append")
        .action(ArgAction::Append)
        .require_equals(true);
}
