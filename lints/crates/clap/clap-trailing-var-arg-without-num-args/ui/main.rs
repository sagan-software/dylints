use clap::Arg;

fn main() {
    let _ = Arg::new("bad").trailing_var_arg(true);
    let _ = Arg::new("good").trailing_var_arg(true).num_args(1..);
}
