use clap::{Arg, ArgAction};

fn main() {
    let _ = Arg::new("bad").trailing_var_arg(true);
    let _ = Arg::new("single")
        .action(ArgAction::Set)
        .trailing_var_arg(true);
    let _ = Arg::new("good").trailing_var_arg(true).num_args(1..);
    let _ = Arg::new("append")
        .action(ArgAction::Append)
        .trailing_var_arg(true);
}
