use clap::Arg;

fn main() {
    let _ = Arg::new("bad").allow_negative_numbers(true);
    let _ = Arg::new("good").allow_negative_numbers(true).num_args(1);
}
