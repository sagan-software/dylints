use clap::Arg;

fn main() {
    let _ = Arg::new("bad").allow_hyphen_values(true);
    let _ = Arg::new("good").allow_hyphen_values(true).num_args(1);
}
