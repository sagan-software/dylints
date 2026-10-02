use clap::Arg;

fn main() {
    let _ = Arg::new("bad").require_equals(true);
    let _ = Arg::new("good").require_equals(true).num_args(1);
}
