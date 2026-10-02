use clap::Arg;

fn main() {
    let _ = Arg::new("bad").long("bad").index(1);
    let _ = Arg::new("option").long("option");
    let _ = Arg::new("positional").index(1);
}
